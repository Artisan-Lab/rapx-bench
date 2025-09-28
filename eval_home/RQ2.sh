#!/bin/bash

# 检查是否传入了 tool 和 type 参数
if [ -z "$1" ] || [ -z "$2" ]; then
  echo "Usage: $0 <tool> <type> [-p]"
  echo "  <tool>   - The tool to run"
  echo "  <type>   - The type to execute (e.g., NPD, BO, Uninit, DF, UAF, ALL)"
  echo "  [-p]     - Optional. Add to enable the -p option for concurrent execution"
  exit 1
fi

# 获取工具、类型和可选的 -p 参数
tool=$1
type=$2
p_option=""

# 检查是否启用 -p 参数
if [ "$3" == "-p" ]; then
  p_option="-p"
fi

# 定义要执行的类型
types=("NPD" "BO" "Uninit" "DF" "UAF" "Other" "ALL")

# 跳转到指定目录
cd /home/varixer/code/EVAL || { echo "Failed to change directory"; exit 1; }

# 如果是 ALL 类型，则遍历前五种类型并执行
if [ "$type" == "ALL" ]; then
  for t in "${types[@]::5}"; do
    echo "Running for tool: $tool and type: $t"
    RUST_LOG=info cargo run -- ../eval_home/script/$tool -o=../eval_home/RQ2/$t -k=$t $p_option >> ../eval_home/RQ2/$tool.txt
  done
else
  # 否则只执行指定的类型
  if [[ " ${types[@]} " =~ " ${type} " ]]; then
    echo "Running for tool: $tool and type: $type"
    RUST_LOG=info cargo run -- ../eval_home/script/$tool -o=../eval_home/RQ2/$type -k=$type $p_option >> ../eval_home/RQ2/$tool.txt
  else
    echo "Invalid type. Valid types are: ${types[@]}"
    exit 1
  fi
fi