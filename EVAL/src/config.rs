use std::path::PathBuf;

use flow::Flows;
use testcase::Testcases;

pub(crate) struct Config {
    pub testcases: Testcases,
    pub flows: Flows,
    pub length: usize,
}

impl Config {
    pub(crate) fn new(config: PathBuf, length: usize) -> Self {
        let testcases = Testcases::from_file(config.join("testcases.yaml"));
        let flows = Flows::from_file(config.join("expressions.yaml"));
        Config {
            testcases,
            flows,
            length,
        }
    }
}

pub(crate) mod testcase {
    use std::{
        ops::{Deref, DerefMut},
        path::Path,
    };

    use log::info;
    use serde::{Deserialize, Serialize};

    use crate::ir::Program;

    #[derive(Debug, Serialize, Deserialize)]
    pub(crate) struct Testcases(Vec<Testcase>);

    impl Testcases {
        pub(crate) fn from_file<P>(path: P) -> Self
        where
            P: AsRef<Path>,
        {
            info!("Testcases from file: {}", path.as_ref().display());
            let cnt = std::fs::read_to_string(path).expect("File not found");
            serde_yaml::from_str(&cnt).expect("File Content Format Error")
        }

        pub(crate) fn filter_by_ty(&self, ty: &str) -> Vec<usize> {
            self.iter()
                .enumerate()
                .filter(|(_, testcase)| testcase.tags.ty == ty)
                .map(|(index, _)| index)
                .collect()
        }
    }

    impl Deref for Testcases {
        type Target = Vec<Testcase>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for Testcases {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub(crate) struct Testcase {
        #[serde(rename = "description")]
        pub desc: String,

        pub tags: Tags,

        pub features: Vec<String>,
        #[serde(rename = "type")]
        pub ty: String,
        #[serde(rename = "value")]
        pub val: String,
        #[serde(rename = "POS")]
        pub pos: Case, // Positive Case
        #[serde(rename = "NEG")]
        pub neg: Case, // Negative Case
    }

    impl Testcase {
        pub(crate) fn into_programs(&self, expr: &String) -> (Program, Program) {
            let pos = Program::new(self.pos.nest(expr), "".to_string());
            let neg = Program::new(self.neg.nest(expr), "".to_string());
            (pos, neg)
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub(crate) struct Tags {
        #[serde(rename = "SP")]
        pub sp: String,
        #[serde(rename = "UB")]
        pub ub: String,
        #[serde(rename = "TY")]
        pub ty: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub(crate) struct Case {
        #[serde(rename = "source")]
        pub src: String,
        pub code: String,
    }

    impl Case {
        /// Nest `expr` to `self.src`, and then nest `self.src` to `self.code`
        pub(crate) fn nest(&self, expr: &String) -> String {
            let source = format!("{{\n{}\n}}", expr.replace("SOURCE!()", &self.src));
            self.code.replace("SOURCE!()", source.trim())
        }
    }
}

pub(crate) mod flow {
    use crate::config::testcase::Testcase;
    use std::{
        ops::{Deref, DerefMut},
        path::Path,
    };

    use log::info;
    use regex::Regex;
    use serde::{Deserialize, Serialize};

    use crate::ir::{Expr, Exprs};

    #[derive(Debug, Serialize, Deserialize)]
    pub(crate) struct Flows(pub Vec<Flow>);

    impl Flows {
        pub(crate) fn from_file<P>(path: P) -> Self
        where
            P: AsRef<Path>,
        {
            info!("Flows from file: {}", path.as_ref().display());
            let cnt = std::fs::read_to_string(path).expect("File not found");
            serde_yaml::from_str(&cnt).expect("File Content Format Error")
        }
    }

    impl Deref for Flows {
        type Target = Vec<Flow>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for Flows {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub(crate) struct Flow {
        pub name: String,
        pub code: String,
    }

    impl Flow {
        pub(crate) fn into_expr(
            &self,
            num: usize,
            src: &Expr,
            exprs: &Exprs,
            case: &Testcase,
        ) -> Expr {
            let mut code = self
                .code
                .replacen("SOURCE!()", &format!("{{\n{}\n}}", &src.code), 1); // SOURCE!() 替换
            code = code.replace("TYPE!()", &case.ty); // TYPE!() 替换
            code = code.replace("VALUE!()", &format!("{{\n{}\n}}", case.val)); // VALUE!() 替换
            code = code.replace("COND!()", "true"); // COND!() 替换
            let length = src.length + 1;
            let mut depth = src.depth;

            // EXPR!() 替换
            let re = Regex::new(r"EXPRE!\((.*?)\)").unwrap();
            code = re
                .replace_all(&code, |caps: &regex::Captures| {
                    // 提取括号内的内容
                    let param = caps[1].to_string();
                    let expr = exprs.random_expr().unwrap();
                    depth = std::cmp::max(depth, expr.depth + 1);
                    format!("{{\n{}\n}}", expr.fill_source(&param)) // 用指定的替换字符串
                })
                .to_string();

            let metadata = src.metadata.clone();
            Expr::new(num, code, length, depth, metadata)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::config::{
        flow::Flows,
        testcase::{Case, Testcases},
    };

    #[test]
    fn test_testcases_from_file() {
        let testcases = Testcases::from_file("config/testcases.yaml");
        assert_eq!(testcases[0].ty, String::from("Layout"));
    }

    #[test]
    fn test_flows_from_file() {
        let flows = Flows::from_file("config/expressions.yaml");
        assert_eq!(flows[0].name, String::from("Function call"));
    }

    #[test]
    fn test_generate() {
        let case = Case {
            src: String::from("Layout::from_size_align(0, 1).unwrap()"),
            code: String::from(
                r#"
    use std::alloc::{alloc, dealloc, Layout};
    fn main() {
        let layout = SOURCE!();
        let ptr = unsafe { alloc(layout) }; // SINK
        unsafe { dealloc(ptr, layout) };
    }"#,
            ),
        };

        let expr = String::from(
            r#"
    fn call(param: TYPE!()) -> TYPE!() {
        EXPRE!(param)
    }
    call(SOURCE!())
        "#,
        );

        println!("{}", case.nest(&expr));
    }
}
