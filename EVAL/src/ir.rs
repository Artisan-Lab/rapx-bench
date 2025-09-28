use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use rand::Rng as _;

pub(crate) struct Exprs(Vec<Expr>);

impl Exprs {
    pub(crate) fn new() -> Self {
        Exprs(Vec::new())
    }

    /// 随机返回 `Exprs` 实例中一个 `Expr` 的共享引用
    pub(crate) fn random_expr(&self) -> Option<&Expr> {
        if self.0.is_empty() {
            None // 如果没有任何元素，返回 None
        } else {
            let mut rng = rand::thread_rng();
            let index = rng.gen_range(0..self.0.len()); // 随机生成索引
            self.0.get(index) // 返回共享引用
        }
    }
}

impl Deref for Exprs {
    type Target = Vec<Expr>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Exprs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Expr {
    pub num: String,
    pub code: String,
    pub length: usize,
    pub depth: usize,
    pub metadata: String,
}

impl Expr {
    pub(crate) fn new(
        num: usize,
        code: String,
        length: usize,
        depth: usize,
        metadata: String,
    ) -> Self {
        let num = format!("{:03}-{}-{}", num, length, depth);
        Expr {
            num,
            code,
            length,
            depth,
            metadata,
        }
    }

    /// SOURCE!()
    pub(crate) fn source() -> Self {
        Expr::new(0, String::from("SOURCE!()"), 0, 0, String::from(""))
    }

    /// SOURCE!() 替换
    pub(crate) fn fill_source(&self, src: &String) -> String {
        self.code.replace("SOURCE!()", src)
    }
}

pub(crate) struct Program {
    pub code: String,
    // pub metadata: String, // 注释格式的程序信息
}

impl Program {
    pub(crate) fn new(code: String, _metadata: String) -> Self {
        // Program { code, metadata }
        Program { code }
    }

    /// Merge `metadata` and `code`
    pub(crate) fn merge(&self) -> String {
        self.code.clone()
    }

    pub(crate) fn into_harness(&self, harness: &PathBuf) {
        std::fs::write(harness.join("src/main.rs"), self.merge()).expect("Failed to write");
    }
}
