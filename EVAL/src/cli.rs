use std::path::PathBuf;

use clap::{builder::PossibleValue, ArgAction, ArgGroup, Parser, ValueEnum};
use eval::Evaluator;
#[derive(Parser)]
#[command(group(ArgGroup::new("testcase_filter").args(&["kind", "indices"]).required(false)))]
pub(crate) struct Cli {
    /// Tools to be evaluated
    tool: PathBuf,

    /// Configuration file
    #[arg(short, long, value_name = "DIR")]
    config: Option<PathBuf>,

    /// Evaluation by vulnerability kind (mutually exclusive with --indices)
    #[arg(short, long, value_name = "TYPE", group = "testcast_filter")]
    kind: Option<Kind>,

    /// Indices of the testcases (mutually exclusive with --kind) [default: ALL]
    #[arg(short, long, value_parser, num_args=1.., group = "testcast_filter")]
    indices: Vec<usize>,

    /// Expression sequence length
    #[arg(short, long, value_name = "NUM", default_value_t = 2)]
    length: usize,

    /// Output path
    #[arg(short, long, value_name = "DIR")]
    output: Option<PathBuf>,

    /// Enable parallel execution [default: false]
    #[arg(short, long, action = ArgAction::SetTrue, default_value_t = false)]
    parallel: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Kind {
    UAF,
    DF,
    BO,
    Uninit,
    NPD,
    Other,
}

impl ValueEnum for Kind {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::UAF, Self::DF, Self::BO, Self::Uninit, Self::NPD, Self::Other]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Kind::UAF => PossibleValue::new("UAF"),
            Kind::DF => PossibleValue::new("DF"),
            Kind::BO => PossibleValue::new("BO"),
            Kind::Uninit => PossibleValue::new("Uninit"),
            Kind::NPD => PossibleValue::new("NPD"),
            Kind::Other => PossibleValue::new("Other"),
        })
    }
}

impl Kind {
    fn as_str(&self) -> &'static str {
        match self {
            Kind::UAF => "UAF",
            Kind::DF => "DF",
            Kind::BO => "BO",
            Kind::Uninit => "Uninit",
            Kind::NPD => "NPD",
            Kind::Other => "Other",
                    }
    }
}

impl Cli {
    pub(crate) fn main(self) {
        let current_dir = std::env::current_dir().unwrap();
        let output = self.output.unwrap_or(current_dir.join("output"));
        let config = self.config.unwrap_or(current_dir.join("config"));
        let mut eval = Evaluator::new(self.tool, config, self.indices, self.length, output);
        if let Some(k) = self.kind {
            eval.set_target_by_ty(k.as_str());
        }

        eval.main(self.parallel);
    }
}
