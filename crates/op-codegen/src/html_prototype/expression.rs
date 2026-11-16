//! Compile authored expressions with Jian's parser and bytecode compiler.
//! The browser consumes constants and stable AOT opcodes as JSON data.

use jian_core::expression::{bytecode::OpCode, Expression};
use jian_ops_schema::pack::PackedOpCode;
use serde_json::{json, Value};

pub(super) fn compile(source: &str) -> Option<Value> {
    let Expression { chunk, .. } = Expression::compile(source).ok()?;
    let ops: Vec<_> = chunk
        .ops
        .into_iter()
        .map(|op| {
            let tag = PackedOpCode::from(&op).tag();
            match op {
                OpCode::PushNum(n) => json!([tag, n]),
                OpCode::PushBool(b) => json!([tag, b]),
                OpCode::PushString(i) | OpCode::PushObjectKey(i) | OpCode::MemberGet(i) => {
                    json!([tag, chunk.strings[i as usize]])
                }
                OpCode::PushScopeRef(i) => json!([tag, chunk.scope_paths[i as usize]]),
                OpCode::MakeArray(n) | OpCode::MakeObject(n) => json!([tag, n]),
                OpCode::JumpIfFalse(n) | OpCode::JumpIfTrue(n) | OpCode::Jump(n) => {
                    json!([tag, n])
                }
                OpCode::CallBuiltin(i, argc) => json!([tag, chunk.strings[i as usize], argc]),
                _ => json!([tag]),
            }
        })
        .collect();
    Some(json!(ops))
}

pub(super) fn from_value(value: &Value) -> Option<Value> {
    compile(value.as_str()?)
}
