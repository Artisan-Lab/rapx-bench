use std::collections::HashMap;

use serde::Serialize;
use serde::Serializer;

use crate::config::flow::Flows;

#[derive(Debug, Clone, Copy)]
pub(crate) struct EvalResults(pub EvalResult, pub EvalResult);

impl Serialize for EvalResults {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let serialized_value = match (self.0, self.1) {
            (EvalResult::Err, _) | (_, EvalResult::Err) => "Error",
            (EvalResult::TP, EvalResult::TN) => "True Positive & Negative",
            (EvalResult::TP, EvalResult::FP) => "False Positive",
            (EvalResult::FN, EvalResult::FP) => "False Positive & Negative",
            (EvalResult::FN, EvalResult::TN) => "False Negative",
            _ => unreachable!(),
        };

        serializer.serialize_str(serialized_value)
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) enum EvalResult {
    Err, // 工具执行出错
    TP,
    FP, // 误报
    FN, // 漏报
    TN,
}

pub(crate) struct EvalMap(HashMap<String, String>);

impl EvalMap {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn insert(&mut self, k: String, v: &str, EvalResults(pos, neg): &EvalResults) {
        let str = match (pos, neg) {
            (EvalResult::Err, _) | (_, EvalResult::Err) => "Error",
            (EvalResult::TP, EvalResult::FP) => "FP",
            (EvalResult::FN, EvalResult::FP) => "FN & FP",
            (EvalResult::FN, EvalResult::TN) => "FN",
            _ => unreachable!(),
        };
        self.0.insert(k, format!("{} {}", v, str));
    }

    pub(crate) fn to_vec(&self, flows: &Flows) -> Vec<String> {
        let mut res = vec!["-".to_string(); flows.len() + 1]; // 0 下标为 最简用例
        if self.0.contains_key("-") {
            res[0] = self.0.get("-").unwrap().to_string();
        } else {
            for (i, flow) in flows.iter().enumerate() {
                if self.0.contains_key(&flow.name) {
                    res[i + 1] = self.0.get(&flow.name).unwrap().to_string(); // 下标 + 1 为对应 flow 的位置
                }
            }
        }
        res
    }
}

pub(crate) mod counter {
    use crate::result::EvalResult;
    use crate::result::EvalResults;

    #[derive(serde::Serialize)]
    pub(crate) struct EvalCounter {
        #[serde(
            rename = "编号",
            serialize_with = "EvalCounter::format_with_leading_zeros"
        )]
        pub idx: usize,
        #[serde(rename = "变体")]
        pub variant_count: usize,
        #[serde(rename = "TP")]
        pub tp_count: usize,
        #[serde(rename = "FN")]
        pub fn_count: usize,
        #[serde(rename = "EP")]
        pub pos_err_count: usize,
        #[serde(rename = "TN")]
        pub tn_count: usize,
        #[serde(rename = "FP")]
        pub fp_count: usize,
        #[serde(rename = "EN")]
        pub neg_err_count: usize,
        #[serde(rename = "RD")]
        pub robust_count: usize,
    }

    impl EvalCounter {
        /// Init Eval Counter (All are 0)
        pub(crate) fn new(idx: usize) -> Self {
            EvalCounter {
                idx,
                variant_count: 0,
                tp_count: 0,
                fp_count: 0,
                pos_err_count: 0,
                fn_count: 0,
                tn_count: 0,
                neg_err_count: 0,
                robust_count: 0,
            }
        }

        /// Count based on res enumeration
        pub(crate) fn count(&mut self, res: &EvalResults) {
            self.variant_count += 1;
            match res.0 {
                EvalResult::Err => self.pos_err_count += 1,
                EvalResult::TP => self.tp_count += 1,
                EvalResult::FN => self.fn_count += 1,
                _ => unreachable!(), // POS Case 不存在 TN 和 FP
            }
            match res.1 {
                EvalResult::Err => self.neg_err_count += 1,
                EvalResult::FP => {
                    if let EvalResult::TP = res.0 {
                        // 召回正例，反例的检测才有意义
                        self.fp_count += 1
                    }
                }
                EvalResult::TN => {
                    if let EvalResult::TP = res.0 {
                        // 召回正例，反例的检测才有意义
                        self.tn_count += 1
                    }
                }
                _ => unreachable!(), // NEG Case 不存在 TP 和 FN
            }
            if let EvalResults(EvalResult::TP, EvalResult::TN) = res {
                self.robust_count += 1;
            }
        }

        // Custom function to serialize numbers with leading zeros
        pub(crate) fn format_with_leading_zeros<S>(
            num: &usize,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let formatted = format!("{:03}", num); // Format number with leading zeros, width = 3
            serializer.serialize_str(&formatted)
        }
    }
}

pub(crate) mod tree {
    use std::{cell::RefCell, collections::HashMap, rc::Rc};

    use serde::{ser::SerializeSeq as _, Serialize, Serializer};

    use crate::{result::EvalResult, EvalResults};

    #[derive(Debug, Serialize)]
    pub(crate) struct EvalNode {
        #[serde(rename = "index")]
        pub name: String,
        #[serde(rename = "result")]
        pub res: EvalResults,
        #[serde(rename = "variants", serialize_with = "EvalNode::serialize_children")]
        pub children: Vec<Rc<RefCell<EvalNode>>>,
    }

    impl EvalNode {
        /// 创建新节点
        pub(crate) fn new(name: &str, res: EvalResults) -> Rc<RefCell<Self>> {
            Rc::new(RefCell::new(Self {
                name: name.to_string(),
                res,
                children: Vec::new(),
            }))
        }

        /// 递归生成 DOT 格式字符串
        pub(crate) fn to_dot(
            &self,
            dot: &mut String,
            parent_id: Option<usize>,
            counter: &mut usize,
        ) {
            let node_id = *counter; // 当前节点的唯一 ID
            *counter += 1;

            // 定义节点的颜色
            let color = match (self.res.0, self.res.1) {
                (EvalResult::Err, _) | (_, EvalResult::Err) => "red", // 意外终止
                (EvalResult::TP, EvalResult::TN) => "green",          // 鲁棒检测
                (EvalResult::TP, EvalResult::FP) => "blue",           // 误报
                (EvalResult::FN, EvalResult::FP) => "gray",           // 漏报 + 误报
                (EvalResult::FN, EvalResult::TN) => "orange",         // 漏报
                _ => unreachable!(),
            };

            // 添加当前节点
            dot.push_str(&format!(
                "node{} [label=\"{}\" style=filled fillcolor={}];\n",
                node_id, self.name, color
            ));

            // 如果有父节点，连接边
            if let Some(parent_id) = parent_id {
                dot.push_str(&format!("node{} -> node{};\n", parent_id, node_id));
            }

            // 递归处理子节点
            for child in &self.children {
                child.borrow().to_dot(dot, Some(node_id), counter);
            }
        }

        pub(crate) fn serialize_children<S>(
            value: &Vec<Rc<RefCell<EvalNode>>>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut seq = serializer.serialize_seq(Some(value.len()))?;
            for rc in value {
                seq.serialize_element(&*rc.borrow())?;
            }
            seq.end()
        }
    }

    pub(crate) struct EvalTree {
        pub root: Option<Rc<RefCell<EvalNode>>>,
        pub node_map: HashMap<String, Rc<RefCell<EvalNode>>>,
    }

    impl EvalTree {
        /// 创建空树
        pub(crate) fn new() -> Self {
            Self {
                root: None,
                node_map: HashMap::new(),
            }
        }

        /// 获取节点数
        pub(crate) fn count_nodes(&self) -> usize {
            self.node_map.len()
        }

        /// 设置根节点
        pub(crate) fn set_root(&mut self, root: Rc<RefCell<EvalNode>>) {
            self.node_map
                .insert(root.borrow().name.clone(), Rc::clone(&root));
            self.root = Some(root);
        }

        /// 根据 name 查找节点
        pub(crate) fn get_node(&self, name: &str) -> Option<Rc<RefCell<EvalNode>>> {
            self.node_map.get(name).cloned()
        }

        /// 添加子节点
        pub(crate) fn add_child(
            &mut self,
            parent_name: &str,
            child_name: &str,
            child_res: EvalResults,
        ) -> Result<(), String> {
            if let Some(parent) = self.get_node(parent_name) {
                let child = EvalNode::new(child_name, child_res);
                parent.borrow_mut().children.push(Rc::clone(&child));
                self.node_map
                    .insert(child_name.to_string(), Rc::clone(&child));
                Ok(())
            } else {
                Err(format!("Parent node '{}' not found", parent_name))
            }
        }

        pub(crate) fn to_dot(&self) -> String {
            let mut dot = String::from("digraph EvalTree {\n");
            dot.push_str("node [shape=ellipse];\n"); // 设置全局节点格式

            if let Some(root) = &self.root {
                let mut counter = 0;
                root.borrow().to_dot(&mut dot, None, &mut counter);
            }

            dot.push('}');
            dot
        }

        // pub(crate) fn to_json(&self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        //     if let Some(root) = &self.root {
        //         serde_json::to_writer_pretty(
        //             File::create(path.join("evalTree.json"))?,
        //             &*root.borrow(),
        //         )
        //         .unwrap();
        //     }
        //     Ok(())
        // }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        #[test]
        fn test() {
            // 创建树并设置根节点
            let mut tree = EvalTree::new();
            let root = EvalNode::new("Root", EvalResults(EvalResult::TP, EvalResult::TN));
            tree.set_root(Rc::clone(&root));

            // 添加子节点
            tree.add_child(
                "Root",
                "Child1",
                EvalResults(EvalResult::TP, EvalResult::TN),
            )
            .unwrap();
            tree.add_child(
                "Root",
                "Child2",
                EvalResults(EvalResult::TP, EvalResult::FP),
            )
            .unwrap();
            tree.add_child(
                "Child1",
                "GrandChild1",
                EvalResults(EvalResult::FN, EvalResult::TN),
            )
            .unwrap();
            tree.add_child(
                "Child2",
                "GrandChild2",
                EvalResults(EvalResult::FN, EvalResult::FP),
            )
            .unwrap();

            // 生成 DOT 文件内容
            let dot_content = tree.to_dot();
            println!("DOT Representation:\n{}", dot_content);

            // 保存到文件
            let dot_path = "tree.dot";
            std::fs::write(dot_path, dot_content).expect("Failed to write DOT file");

            // 调用 Graphviz 将 DOT 转为图片
            let output = std::process::Command::new("dot")
                .args(&["-Tpng", "-o", "tree.png", dot_path])
                .output()
                .expect("Failed to execute Graphviz");
            if output.status.success() {
                println!("EvalTree image generated: tree.png");
            } else {
                eprintln!(
                    "Error generating image: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
}

pub(crate) mod summary {
    use crate::result::counter::EvalCounter;
    use core::fmt;

    use tabled::Tabled;

    #[derive(Default)]
    pub(crate) struct Metric {
        normal: usize,
        absolute: usize,
    }

    impl fmt::Display for Metric {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // 定义格式为 "normal (absolute)"
            write!(f, "{} ({})", self.normal, self.absolute)
        }
    }

    impl Metric {
        pub(crate) fn count(&mut self, src: usize, tar: usize) {
            if src != 0 {
                self.normal += 1;
                if src == tar {
                    self.absolute += 1;
                }
            }
        }
    }

    #[derive(Default, Tabled)]
    pub(crate) struct EvalSummary {
        #[tabled(rename = "工具")]
        pub tool: String,
        #[tabled(rename = "用例数")]
        pub case_num: usize,
        #[tabled(rename = "真正例 (TP)")]
        pub true_positive: Metric,
        #[tabled(rename = "漏报 (FN)")]
        pub false_negative: Metric,
        #[tabled(rename = "错误 (EP)")]
        pub positive_error: Metric,
        #[tabled(rename = "真反例 (TN)")]
        pub true_negative: Metric,
        #[tabled(rename = "误报 (FP)")]
        pub false_postive: Metric,
        #[tabled(rename = "错误 (EN)")]
        pub negative_error: Metric,
        #[tabled(rename = "鲁棒检测 (RD)")]
        pub robust_detection: Metric,
    }

    impl EvalSummary {
        pub(crate) fn new(tool: String) -> Self {
            Self {
                tool,
                ..Default::default()
            }
        }
        pub(crate) fn summary(tool: String, counters: &[EvalCounter]) -> Self {
            let mut summary = EvalSummary::new(tool);
            summary.case_num = counters.len();
            counters.iter().for_each(|s| {
                summary
                    .robust_detection
                    .count(s.robust_count, s.variant_count);
                summary.true_positive.count(s.tp_count, s.variant_count);
                summary.false_negative.count(s.fn_count, s.variant_count);
                summary
                    .positive_error
                    .count(s.pos_err_count, s.variant_count);
                summary.false_postive.count(s.fp_count, s.variant_count);
                summary
                    .true_negative
                    .count(s.tn_count, s.variant_count - s.fn_count);
                summary
                    .negative_error
                    .count(s.neg_err_count, s.variant_count);
            });
            summary
        }

        pub(crate) fn report(&self) -> String {
            format!(
                r#"在 {} 个基础用例的正例中有 {} 个正例为漏报和 {} 个正例为错误，而剩余的 {} 个基础用例的反例中有 {} 个反例为误报且 {} 个反例为错误。也就是说，总共有 {} 个基础用例被成功检测出正例并过滤掉反例（相对鲁棒检测），可进行后续的变体测试。
而后续对剩余的 {} 组基础用例的变体测试中，正例中有 {} 组被绝对检测出来、 {} 组包含漏报且 {} 组错误，而反例中有 {} 组被绝对过滤掉、 {} 组包含误报且有 {} 组包含错误。因此，只有 {} 个基础用例的所有变体被成功检测出正例并过滤掉反例（绝对鲁棒检测）。
"#,
                self.case_num,
                self.false_negative.absolute,
                self.positive_error.absolute,
                self.true_positive.normal,
                self.false_postive.absolute,
                self.negative_error.absolute,
                self.robust_detection.normal,
                self.robust_detection.normal,
                self.true_positive.absolute - self.false_postive.absolute,
                self.false_negative.normal - self.false_negative.absolute,
                self.positive_error.normal - self.positive_error.absolute,
                self.true_negative.absolute,
                self.false_postive.normal - self.false_postive.absolute,
                self.negative_error.normal - self.negative_error.absolute,
                self.robust_detection.absolute
            )
        }
    }
}
