mod config;
mod ir;
mod result;
mod utils;

use config::Config;
use ir::{Expr, Exprs, Program};
use log::{error, info};
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use result::{
    counter::EvalCounter,
    summary::EvalSummary,
    tree::{EvalNode, EvalTree},
    EvalMap, EvalResult, EvalResults,
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    process::{Command, Output},
};
use tabled::Table;

pub struct Evaluator {
    executor: Executor,
    config: Config,
    targets: Vec<usize>,
    output: PathBuf,
}

impl Evaluator {
    pub fn new(
        tool: PathBuf,
        config: PathBuf,
        targets: Vec<usize>,
        length: usize,
        output: PathBuf,
    ) -> Self {
        env_logger::init();
        utils::is_executable(&tool);
        let output = output.join(tool.file_stem().unwrap());
        let harness = output.join("harness");
        std::fs::create_dir_all(&output).expect(" std::fs::create_dir_all failed");
        Evaluator {
            executor: Executor::new(tool, harness),
            config: Config::new(config, length),
            targets,
            output,
        }
    }

    pub fn set_target_by_ty(&mut self, ty: &str) {
        self.targets = self.config.testcases.filter_by_ty(ty);
    }

    pub fn main(&self, parallel: bool) {
        // 确保 targets 的生命周期独立
        let targets = if self.targets.is_empty() {
            (0..self.config.testcases.len()).collect()
        } else {
            self.targets.clone()
        };
        let results: Vec<_> = if parallel {
            // 拆分任务为大小为 5 的块，并行处理每个块
            targets
                .chunks(5) // 按 5 个任务为一组拆分
                .flat_map(|chunk| {
                    // 对每个块并行处理
                    chunk
                        .par_iter() // 使用并行迭代器
                        .map(|&idx| self.evaluate_one(idx)) // 并行执行任务
                        .collect::<Vec<_>>() // 将块的结果收集为 Vec
                })
                .collect() // 将所有块的结果合并为一个 Vec
        } else {
            // 逐个处理每个任务
            targets
                .iter() // 使用并行迭代器
                .map(|&idx| self.evaluate_one(idx))
                .collect()
        };

        let (counters, maps): (Vec<EvalCounter>, Vec<EvalMap>) =
            results.into_iter().map(|(c, m)| (c, m)).unzip();
        // 导出 EvalCounter
        utils::serialize_to_csv(&counters, self.output.join("EvalCounter.csv")).unwrap();
        let tables = maps
            .iter()
            .map(|m| m.to_vec(&self.config.flows))
            .collect::<Vec<Vec<String>>>();

        utils::serialize_to_csv(&tables, self.output.join("EvalMap.csv")).unwrap();
        // 输出 EvalSummary
        let summary = EvalSummary::summary(self.executor.name(), &counters);
        println!("{}\n{}", Table::new(vec![&summary]), summary.report());
    }

    pub(crate) fn evaluate_one(&self, idx: usize) -> (EvalCounter, EvalMap) {
        if idx >= self.config.testcases.len() {
            error!(
                "Error: Index {} is out of bounds. Valid range is 0-{}",
                idx,
                self.config.testcases.len() - 1
            );
            std::process::exit(1);
        } else {
            utils::generate_harness(self.executor.harness.join(format!("harness-{}", idx)));
            self.evaluate(idx)
        }
    }

    pub(crate) fn evaluate(&self, idx: usize) -> (EvalCounter, EvalMap) {
        let process = |expr: &Expr, (pos, neg): (Program, Program)| -> EvalResults {
            // 写入文件
            info!(
                "Write testcase-{:03} with expression-{} into file system",
                idx, &expr.num
            );
            utils::write(
                self.output
                    .join(format!("testcase-{:03}", idx))
                    .join(&expr.num),
                (&pos, &neg),
            );

            // 执行评估
            let outputs = (
                self.executor.execute(idx, pos),
                self.executor.execute(idx, neg),
            );

            utils::evaluate(outputs)
        };

        // 获取要评估的 testcase
        let testcase = &self.config.testcases[idx];
        // 初始化 EvalCounter 和 EvalTree
        let mut counter = EvalCounter::new(idx);
        let mut tree = EvalTree::new();
        let mut map = EvalMap::new();

        // 评估 testcase
        let src_expr = Expr::source();
        let programs = testcase.into_programs(&src_expr.code);
        let res = process(&src_expr, programs);
        counter.count(&res);

        // BFS 遍历所有可行的 flow 的组合方案
        let root = EvalNode::new(&src_expr.num, res);
        tree.set_root(root);

        if let EvalResults(EvalResult::TP, EvalResult::TN) = res {
            // 变体评估
            // 评估嵌套 flow 后的 testcase 变体
            if self.config.length > 0 {
                // exprs 初始化
                let mut exprs = Exprs::new();
                exprs.push(Expr::source());

                // sources 队列
                let mut sources = VecDeque::new();
                sources.push_back(Expr::source());

                // flows 队列
                let mut flows = VecDeque::from(self.config.flows.0.clone());

                while !sources.is_empty() {
                    let src = sources.pop_front().unwrap();
                    flows.retain(|flow| {
                        let expr: Expr = flow.into_expr(tree.count_nodes(), &src, &exprs, testcase);
                        let programs = testcase.into_programs(&expr.code);
                        let res = process(&expr, programs);
                        counter.count(&res); // 统计
                        tree.add_child(&src.num, &expr.num, res).unwrap(); // 插入评估树

                        if let EvalResults(EvalResult::TP, EvalResult::TN) = res {
                            if expr.length < self.config.length {
                                sources.push_back(expr.clone());
                                exprs.push(expr);
                            }
                            true // 保留 flow
                        } else {
                            map.insert(flow.name.clone(), &expr.num, &res);
                            false // 移除 flow
                        }
                    });
                }
            }
        } else {
            // 最简用例未通过
            map.insert("-".to_string(), &testcase.tags.ty, &res);
        }
        // tree.to_json(self.output.join(format!("testcase-{:03}", idx))).unwrap();
        utils::generate_image_from_dot(
            &tree.to_dot(),
            self.output
                .join(format!("testcase-{:03}", idx))
                .join("evalTree.png"),
        )
        .unwrap();
        return (counter, map);
    }
}

pub(crate) struct Executor {
    pub tool: PathBuf,
    pub harness: PathBuf,
}

impl Executor {
    pub(crate) fn new(tool: PathBuf, harness: PathBuf) -> Self {
        std::fs::create_dir_all(&harness).expect(" std::fs::create_dir_all failed");
        Executor { tool, harness }
    }

    pub(crate) fn name(&self) -> String {
        self.tool
            .file_stem()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap()
    }

    pub(crate) fn execute(&self, idx: usize, program: Program) -> Output {
        program.into_harness(&self.harness.join(format!("harness-{}", idx)));
        Command::new(&self.tool)
            .arg(&self.harness.join(format!("harness-{}", idx)))
            .output()
            .expect("Tool failed to execute")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_execute() {
        let executor = Executor::new(
            "../tools/eval_home/shell/Safedrop".into(),
            "./output/harness".into(),
        );
        let program = Program::new(
            r#"
fn main() {
    println!("Hello, world!");
}
"#
            .to_string(),
            "".to_string(),
        );

        let output = executor.execute(0, program);
        println!("{:#?}", output);
    }
}
