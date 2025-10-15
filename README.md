# rapx-bench
This program is to benchmarking the the detection capabilities and applicability of Rust static anaylysis and model checking tools.
There are two main folders in this program, EVAL and eval_home. In EVAL, there're rust programs of the benchmarking framework and tesecases while other parts like scripts of tools, harness folders where testcases will be put in, are in the eval_home folder.

Check out supported options with -help or -h:

cargo run -- -h

Usage: eval [OPTIONS] <TOOL>

Arguments:
  <TOOL>  Tools to be evaluated

Options:
  -c, --config <DIR>          Configuration file
  -k, --kind <TYPE>           Evaluation by vulnerability kind (mutually exclusive with --indices) [possible values: UAF, DF, BO, Uninit, NPD, Other]
  -i, --indices <INDICES>...  Indices of the testcases (mutually exclusive with --kind) [default: ALL]
  -l, --length <NUM>          Expression sequence length [default: 2]
  -o, --output <DIR>          Output path
  -p, --parallel              Enable parallel execution [default: false]
  -h, --help                  Print help

To use this benchmarking framework, you need to enter EVAL and run the command like:

RUST_LOG=info cargo run -- ../eval_home/script/Safedrop -o=../eval_home/TT -i=16 -p -l=0

in the terminal there will be outputs:
+----------+--------+-------------+-----------+-----------+-------------+-----------+-----------+---------------+
| 工具     | 用例数 | 真正例 (TP) | 漏报 (FN) | 错误 (EP) | 真反例 (TN) | 误报 (FP) | 错误 (EN) | 鲁棒检测 (RD) |
+----------+--------+-------------+-----------+-----------+-------------+-----------+-----------+---------------+
| Safedrop | 1      | 0 (0)       | 1 (1)     | 0 (0)     | 0 (0)       | 0 (0)     | 0 (0)     | 0 (0)         |
+----------+--------+-------------+-----------+-----------+-------------+-----------+-----------+---------------+
在 1 个基础用例的正例中有 1 个正例为漏报和 0 个正例为错误，而剩余的 0 个基础用例的反例中有 0 个反例为误报且 0 个反例为错误。也就是说，总共有 0 个基础用例被成功检测出正例并过滤掉反例（相对鲁棒检测），可进行后续的变体测试。
而后续对剩余的 0 组基础用例的变体测试中，正例中有 0 组被绝对检测出来、 0 组包含漏报且 0 组错误，而反例中有 0 组被绝对过滤掉、 0 组包含误报且有 0 组包含错误。因此，只有 0 个基础用例的所有变体被成功检测出正例并过滤掉反例（绝对鲁棒检测）。

We've already made one script for Rust static analysis tool SafeDrop(now embedded in RAPX) in the scripts folder. Make sure the settings in the script matches the version of the tool you installed.
