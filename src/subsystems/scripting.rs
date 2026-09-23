//! Bounded deterministic scripting VM.
//!
//! The VM is intentionally tiny and dependency-free. It establishes the
//! subsystem contract and instruction-budget guard before a richer bytecode
//! format or external language bridge is introduced.

use crate::kernel::{KernelContext, Subsystem};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    Push(i64),
    Add,
    Sub,
    Mul,
    Halt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptError {
    StackOverflow,
    StackUnderflow,
    InstructionBudgetExceeded,
    AllocationFailed,
}

pub struct ScriptingSubsystem {
    program: Vec<Instruction>,
    stack: Vec<i64>,
    instruction_budget: usize,
    pc: usize,
    halted: bool,
    last_error: Option<ScriptError>,
}

impl ScriptingSubsystem {
    pub fn with_capacity(
        program_capacity: usize,
        stack_capacity: usize,
        instruction_budget: usize,
    ) -> Self {
        Self::try_with_capacity(program_capacity, stack_capacity, instruction_budget)
            .expect("scripting allocation failed")
    }

    pub fn try_with_capacity(
        program_capacity: usize,
        stack_capacity: usize,
        instruction_budget: usize,
    ) -> Result<Self, ScriptError> {
        let mut program = Vec::new();
        program
            .try_reserve_exact(program_capacity)
            .map_err(|_| ScriptError::AllocationFailed)?;
        let mut stack = Vec::new();
        stack
            .try_reserve_exact(stack_capacity)
            .map_err(|_| ScriptError::AllocationFailed)?;
        Ok(Self {
            program,
            stack,
            instruction_budget,
            pc: 0,
            halted: false,
            last_error: None,
        })
    }

    pub fn load(&mut self, program: &[Instruction]) -> bool {
        if program.len() > self.program.capacity() {
            return false;
        }
        self.program.clear();
        self.program.extend_from_slice(program);
        self.stack.clear();
        self.pc = 0;
        self.halted = false;
        self.last_error = None;
        true
    }

    pub fn stack(&self) -> &[i64] {
        &self.stack
    }
    pub fn last_error(&self) -> Option<ScriptError> {
        self.last_error
    }

    fn execute(&mut self) {
        let mut executed = 0;
        while !self.halted && self.pc < self.program.len() {
            if executed == self.instruction_budget {
                self.last_error = Some(ScriptError::InstructionBudgetExceeded);
                return;
            }
            let instruction = self.program[self.pc];
            self.pc += 1;
            executed += 1;
            match instruction {
                Instruction::Push(value) => {
                    if self.stack.len() == self.stack.capacity() {
                        self.last_error = Some(ScriptError::StackOverflow);
                        return;
                    }
                    self.stack.push(value);
                }
                Instruction::Add | Instruction::Sub | Instruction::Mul => {
                    let Some(rhs) = self.stack.pop() else {
                        self.last_error = Some(ScriptError::StackUnderflow);
                        return;
                    };
                    let Some(lhs) = self.stack.pop() else {
                        self.last_error = Some(ScriptError::StackUnderflow);
                        return;
                    };
                    let value = match instruction {
                        Instruction::Add => lhs.saturating_add(rhs),
                        Instruction::Sub => lhs.saturating_sub(rhs),
                        Instruction::Mul => lhs.saturating_mul(rhs),
                        Instruction::Push(_) | Instruction::Halt => unreachable!(),
                    };
                    self.stack.push(value);
                }
                Instruction::Halt => self.halted = true,
            }
        }
    }
}

impl Default for ScriptingSubsystem {
    fn default() -> Self {
        Self::with_capacity(256, 64, 1024)
    }
}

impl Subsystem for ScriptingSubsystem {
    fn name(&self) -> &'static str {
        "scripting"
    }
    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }
    fn init(&mut self, _ctx: &mut KernelContext<'_>) {}
    fn tick(&mut self, _ctx: &mut KernelContext<'_>, _dt_ns: u64) {
        if !self.halted && self.last_error.is_none() {
            self.execute();
        }
    }
    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_arithmetic_without_allocating_during_execution() {
        let mut vm = ScriptingSubsystem::with_capacity(8, 4, 8);
        assert!(vm.load(&[
            Instruction::Push(6),
            Instruction::Push(7),
            Instruction::Mul,
            Instruction::Halt
        ]));
        vm.execute();
        assert_eq!(vm.stack(), &[42]);
        assert_eq!(vm.last_error(), None);
    }

    #[test]
    fn instruction_budget_stops_non_terminating_programs() {
        let mut vm = ScriptingSubsystem::with_capacity(4, 4, 2);
        assert!(vm.load(&[Instruction::Push(1), Instruction::Push(2), Instruction::Add]));
        vm.execute();
        assert_eq!(
            vm.last_error(),
            Some(ScriptError::InstructionBudgetExceeded)
        );
    }
}
