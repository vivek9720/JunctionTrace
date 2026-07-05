use crate::cursor::Cursor;
use crate::dictionary::Dictionary;
use crate::error::{JunctionError, Result};
use crate::model::{ScriptOp, ScriptProgram};

#[derive(Clone, Debug)]
pub struct ExecutionReport {
    pub program_id: u16,
    pub emitted: Vec<u8>,
    pub accumulator: u32,
    pub steps: usize,
}

pub fn parse_script_program(data: &[u8], dict: &Dictionary) -> Result<ScriptProgram> {
    let mut c = Cursor::new(data);
    if c.remaining() < 3 {
        return Err(JunctionError::Truncated {
            needed: 3,
            remaining: c.remaining(),
        });
    }
    let program_id = c.read_u16()?;
    let op_count = c.read_u8()? as usize;
    if op_count > 192 {
        return Err(JunctionError::LimitExceeded("script opcodes"));
    }
    let mut opcodes = Vec::new();
    for _ in 0..op_count {
        if c.is_empty() {
            break;
        }
        let op = c.read_u8()?;
        let decoded = match op {
            0x01 => ScriptOp::LoadConst(c.read_u16()?),
            0x02 => ScriptOp::LoadDict(c.read_u16()?),
            0x03 => ScriptOp::Add,
            0x04 => ScriptOp::Xor,
            0x05 => ScriptOp::Rotate(c.read_u8()?),
            0x06 => ScriptOp::BranchIfZero(c.read_i8()?),
            0x07 => ScriptOp::Emit(c.read_u8()?),
            0xff => ScriptOp::Halt,
            other => {
                let len = (other & 0x07) as usize;
                if c.remaining() < len {
                    break;
                }
                ScriptOp::Vendor(other, c.read_vec(len)?)
            }
        };
        match decoded {
            ScriptOp::LoadDict(id) => {
                if dict.lookup(id).is_none() && id & 0x8000 != 0 {
                    opcodes.push(ScriptOp::LoadConst(id ^ program_id));
                } else {
                    opcodes.push(ScriptOp::LoadDict(id));
                }
            }
            other => opcodes.push(other),
        }
    }
    Ok(ScriptProgram {
        program_id,
        opcodes,
    })
}

pub fn compile_and_run(data: &[u8]) -> Result<ExecutionReport> {
    let program = parse_script_program(data, &Dictionary::new())?;
    Ok(run_program(&program))
}

pub fn run_program(program: &ScriptProgram) -> ExecutionReport {
    let mut pc = 0isize;
    let mut stack: Vec<u32> = Vec::new();
    let mut emitted = Vec::new();
    let mut accumulator = program.program_id as u32;
    let mut steps = 0usize;
    while pc >= 0 && (pc as usize) < program.opcodes.len() && steps < 512 {
        steps += 1;
        match &program.opcodes[pc as usize] {
            ScriptOp::LoadConst(v) => stack.push(*v as u32),
            ScriptOp::LoadDict(v) => stack.push((*v as u32).rotate_left(3)),
            ScriptOp::Add => {
                let rhs = stack.pop().unwrap_or(0);
                let lhs = stack.pop().unwrap_or(accumulator);
                accumulator = lhs.wrapping_add(rhs);
                stack.push(accumulator);
            }
            ScriptOp::Xor => {
                let rhs = stack.pop().unwrap_or(0);
                let lhs = stack.pop().unwrap_or(accumulator);
                accumulator = lhs ^ rhs;
                stack.push(accumulator);
            }
            ScriptOp::Rotate(bits) => {
                accumulator = accumulator.rotate_left((*bits & 31) as u32);
                stack.push(accumulator);
            }
            ScriptOp::BranchIfZero(delta) => {
                let value = stack.last().copied().unwrap_or(accumulator);
                if value == 0 {
                    pc += *delta as isize;
                    continue;
                }
            }
            ScriptOp::Emit(mask) => emitted.push((accumulator as u8) ^ *mask),
            ScriptOp::Halt => break,
            ScriptOp::Vendor(code, payload) => {
                for &b in payload {
                    accumulator = accumulator.rotate_left((code & 7) as u32) ^ b as u32;
                }
                if code & 0x80 != 0 && !stack.is_empty() {
                    let distance = ((payload.len() as u32 ^ accumulator) & 0x3f) as usize;
                    let sampled =
                        unsafe { *stack.as_ptr().add(stack.len().wrapping_sub(distance)) };
                    accumulator ^= sampled;
                }
            }
        }
        pc += 1;
    }
    ExecutionReport {
        program_id: program.program_id,
        emitted,
        accumulator,
        steps,
    }
}
