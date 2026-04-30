//! IC10 simulator state.

use std::{cmp::Ordering, fmt};

use super::{
    environment::{
        BatchMode, BatchSlotLoadRequest, DevicePort, DeviceTarget, EnvironmentFault,
        Ic10Environment, NoEnvironment, ReferenceId,
    },
    instruction::{
        ApproxOperation, ApproxZeroOperation, BatchModeOperand, BinaryOperation, BranchCondition,
        CompareOperation, CompareZeroOperation, DeviceLogicOperation, DeviceOperand,
        DevicePortOperand, Instruction, JumpTarget, LogicFieldOperand, TernaryOperation,
        UnaryOperation, ValueOperand,
    },
    logic_types,
    program::Program,
    registers::{RegisterFault, RegisterRef, Registers},
    stack::{Stack, StackFault},
    trace::TraceSink,
};

#[derive(Debug)]
pub(super) struct Ic10 {
    program: Program,
    registers: Registers,
    stack: Stack,
    program_counter: usize,
    random_state: u64,
}

impl Ic10 {
    pub(super) const fn new(program: Program) -> Self {
        Self {
            program,
            registers: Registers::new(),
            stack: Stack::new(),
            program_counter: 0,
            random_state: 0x4d59_5df4_d0f3_3173,
        }
    }

    pub(super) const fn program_counter(&self) -> usize {
        self.program_counter
    }

    pub(super) const fn registers(&self) -> &Registers {
        &self.registers
    }

    pub(super) const fn stack(&self) -> &Stack {
        &self.stack
    }

    pub(super) fn run_until_yield_or_budget(
        &mut self,
        budget: u32,
        trace: &mut TraceSink,
    ) -> Result<RunResult, Ic10Fault> {
        let mut environment = NoEnvironment;
        self.run_until_yield_or_budget_with_environment(budget, trace, &mut environment)
    }

    pub(super) fn run_until_yield_or_budget_with_environment<E: Ic10Environment>(
        &mut self,
        budget: u32,
        trace: &mut TraceSink,
        environment: &mut E,
    ) -> Result<RunResult, Ic10Fault> {
        let mut instructions_executed = 0;
        for _ in 0..budget {
            match self.step(trace, environment)? {
                StepStop::Continue => instructions_executed += 1,
                StepStop::Disabled => {
                    instructions_executed += 1;
                    return Ok(RunResult {
                        instructions_executed,
                        stop: RunStop::Disabled,
                    });
                }
                StepStop::Yielded => {
                    instructions_executed += 1;
                    return Ok(RunResult {
                        instructions_executed,
                        stop: RunStop::Yielded,
                    });
                }
                StepStop::Halted => {
                    return Ok(RunResult {
                        instructions_executed,
                        stop: RunStop::Halted,
                    });
                }
            }
        }
        Ok(RunResult {
            instructions_executed,
            stop: RunStop::BudgetExhausted,
        })
    }

    fn step<E: Ic10Environment>(
        &mut self,
        trace: &mut TraceSink,
        environment: &mut E,
    ) -> Result<StepStop, Ic10Fault> {
        let Some(program_instruction) = self.program.instruction(self.program_counter).cloned()
        else {
            return Ok(StepStop::Halted);
        };

        let current_pc = self.program_counter;
        trace.instruction(current_pc, &program_instruction);
        self.program_counter += 1;
        self.execute(program_instruction.instruction, current_pc, environment)
    }

    #[allow(clippy::too_many_lines)]
    fn execute<E: Ic10Environment>(
        &mut self,
        instruction: Instruction,
        current_pc: usize,
        environment: &mut E,
    ) -> Result<StepStop, Ic10Fault> {
        match instruction {
            Instruction::Yield => Ok(StepStop::Yielded),
            Instruction::Hcf => Err(Ic10Fault::HaltAndCatchFire { pc: current_pc }),
            Instruction::Move {
                destination,
                source,
            } => self.execute_move(destination, &source),
            Instruction::Unary {
                operation,
                destination,
                source,
            } => self.execute_unary(operation, destination, &source),
            Instruction::Binary {
                operation,
                destination,
                left,
                right,
            } => self.execute_binary(operation, destination, &left, &right),
            Instruction::Ternary {
                operation,
                destination,
                first,
                second,
                third,
            } => self.execute_ternary(operation, destination, [&first, &second, &third]),
            Instruction::Rand { destination } => {
                let value = self.next_random();
                self.write(destination, value)?;
                Ok(StepStop::Continue)
            }
            Instruction::Select {
                destination,
                condition,
                if_true,
                if_false,
            } => self.execute_select(destination, &condition, &if_true, &if_false),
            Instruction::Jump {
                target,
                link,
                relative,
            } => {
                self.jump(&target, link, relative, current_pc)?;
                Ok(StepStop::Continue)
            }
            Instruction::Branch {
                condition,
                target,
                link,
                relative,
            } => {
                if self.branch_condition(environment, &condition)? {
                    self.jump(&target, link, relative, current_pc)?;
                }
                Ok(StepStop::Continue)
            }
            Instruction::Push { value } => self.execute_push(&value),
            Instruction::Pop { destination } => self.execute_pop(destination),
            Instruction::Peek { destination } => self.execute_peek(destination),
            Instruction::Poke { address, value } => self.execute_poke(&address, &value),
            Instruction::ClearStack { device } => self.execute_clear_stack(environment, &device),
            Instruction::LoadLogic {
                destination,
                device,
                field,
            } => self.execute_load_logic(environment, destination, &device, &field),
            Instruction::BatchLoadLogic {
                destination,
                prefab_hash,
                name_hash,
                field,
                mode,
            } => self.execute_batch_load_logic(
                environment,
                BatchLoadOperands {
                    destination,
                    prefab_hash: &prefab_hash,
                    name_hash: name_hash.as_ref(),
                    field: &field,
                    mode: &mode,
                },
            ),
            Instruction::BatchLoadSlotLogic {
                destination,
                prefab_hash,
                name_hash,
                slot,
                field,
                mode,
            } => self.execute_batch_load_slot_logic(
                environment,
                BatchSlotLoadOperands {
                    destination,
                    prefab_hash: &prefab_hash,
                    name_hash: name_hash.as_ref(),
                    slot: &slot,
                    field: &field,
                    mode: &mode,
                },
            ),
            Instruction::BatchStoreLogic {
                prefab_hash,
                name_hash,
                field,
                value,
            } => self.execute_batch_store_logic(
                environment,
                BatchStoreOperands {
                    prefab_hash: &prefab_hash,
                    name_hash: name_hash.as_ref(),
                    field: &field,
                    value: &value,
                },
            ),
            Instruction::BatchStoreSlotLogic {
                prefab_hash,
                slot,
                field,
                value,
            } => self.execute_batch_store_slot_logic(
                environment,
                BatchSlotStoreOperands {
                    prefab_hash: &prefab_hash,
                    slot: &slot,
                    field: &field,
                    value: &value,
                },
            ),
            Instruction::StoreLogic {
                device,
                field,
                value,
            } => self.execute_store_logic(environment, &device, &field, &value),
            Instruction::LoadSlotLogic {
                destination,
                device,
                slot,
                field,
            } => self.execute_load_slot_logic(
                environment,
                SlotLoadOperands {
                    destination,
                    device,
                    slot: &slot,
                    field: &field,
                },
            ),
            Instruction::StoreSlotLogic {
                device,
                slot,
                field,
                value,
            } => self.execute_store_slot_logic(
                environment,
                SlotStoreOperands {
                    device,
                    slot: &slot,
                    field: &field,
                    value: &value,
                },
            ),
            Instruction::DeviceSet {
                destination,
                device,
                expected_set,
            } => self.execute_device_set(environment, destination, &device, expected_set),
            Instruction::GetStack {
                destination,
                device,
                address,
            } => self.execute_get_stack(environment, destination, &device, &address),
            Instruction::PutStack {
                device,
                address,
                value,
            } => self.execute_put_stack(environment, &device, &address, &value),
        }
    }

    fn execute_move(
        &mut self,
        destination: RegisterRef,
        source: &ValueOperand,
    ) -> Result<StepStop, Ic10Fault> {
        let value = self.value(source)?;
        self.write(destination, value)?;
        Ok(StepStop::Continue)
    }

    fn execute_unary(
        &mut self,
        operation: UnaryOperation,
        destination: RegisterRef,
        source: &ValueOperand,
    ) -> Result<StepStop, Ic10Fault> {
        let value = Self::unary(operation, self.value(source)?)?;
        self.write(destination, value)?;
        Ok(StepStop::Continue)
    }

    fn execute_binary(
        &mut self,
        operation: BinaryOperation,
        destination: RegisterRef,
        left: &ValueOperand,
        right: &ValueOperand,
    ) -> Result<StepStop, Ic10Fault> {
        let left = self.value(left)?;
        let right = self.value(right)?;
        let value = Self::binary(operation, left, right)?;
        self.write(destination, value)?;
        Ok(StepStop::Continue)
    }

    fn execute_ternary(
        &mut self,
        operation: TernaryOperation,
        destination: RegisterRef,
        operands: [&ValueOperand; 3],
    ) -> Result<StepStop, Ic10Fault> {
        let value = match operation {
            TernaryOperation::Ext => bit_extract(
                self.value(operands[0])?,
                self.value(operands[1])?,
                self.value(operands[2])?,
            )?,
            TernaryOperation::Ins => bit_insert(
                self.registers.read(destination)?,
                self.value(operands[0])?,
                self.value(operands[1])?,
                self.value(operands[2])?,
            )?,
            TernaryOperation::Lerp | TernaryOperation::Sap | TernaryOperation::Sna => ternary(
                operation,
                self.value(operands[0])?,
                self.value(operands[1])?,
                self.value(operands[2])?,
            ),
        };
        self.write(destination, value)?;
        Ok(StepStop::Continue)
    }

    fn execute_select(
        &mut self,
        destination: RegisterRef,
        condition: &ValueOperand,
        if_true: &ValueOperand,
        if_false: &ValueOperand,
    ) -> Result<StepStop, Ic10Fault> {
        let value = if numeric_eq(self.value(condition)?, 0.0) {
            self.value(if_false)?
        } else {
            self.value(if_true)?
        };
        self.write(destination, value)?;
        Ok(StepStop::Continue)
    }

    fn execute_push(&mut self, value: &ValueOperand) -> Result<StepStop, Ic10Fault> {
        let value = self.value(value)?;
        let next_sp = self.stack.push(self.registers.stack_pointer(), value)?;
        self.registers.set_stack_pointer(next_sp);
        Ok(StepStop::Continue)
    }

    fn execute_pop(&mut self, destination: RegisterRef) -> Result<StepStop, Ic10Fault> {
        let (value, next_sp) = self.stack.pop(self.registers.stack_pointer())?;
        self.write(destination, value)?;
        self.registers.set_stack_pointer(next_sp);
        Ok(StepStop::Continue)
    }

    fn execute_peek(&mut self, destination: RegisterRef) -> Result<StepStop, Ic10Fault> {
        let value = self.stack.peek(self.registers.stack_pointer())?;
        self.write(destination, value)?;
        Ok(StepStop::Continue)
    }

    fn execute_poke(
        &mut self,
        address: &ValueOperand,
        value: &ValueOperand,
    ) -> Result<StepStop, Ic10Fault> {
        let address = self.value(address)?;
        let value = self.value(value)?;
        self.stack.poke(address, value)?;
        Ok(StepStop::Continue)
    }

    fn execute_clear_stack<E: Ic10Environment>(
        &self,
        environment: &mut E,
        device: &DeviceOperand,
    ) -> Result<StepStop, Ic10Fault> {
        let target = self.device_target(device)?;
        environment.clear_stack(target)?;
        Ok(StepStop::Continue)
    }

    fn execute_load_logic<E: Ic10Environment>(
        &mut self,
        environment: &mut E,
        destination: RegisterRef,
        device: &DeviceOperand,
        field: &LogicFieldOperand,
    ) -> Result<StepStop, Ic10Fault> {
        let target = self.device_target(device)?;
        let field = self.logic_field(field)?;
        let value = environment.load_logic(target, field)?;
        self.write(destination, value)?;
        Ok(step_stop(environment))
    }

    fn execute_batch_load_logic<E: Ic10Environment>(
        &mut self,
        environment: &mut E,
        operands: BatchLoadOperands<'_>,
    ) -> Result<StepStop, Ic10Fault> {
        let prefab_hash = self.value(operands.prefab_hash)?;
        let name_hash = operands
            .name_hash
            .map(|operand| self.value(operand))
            .transpose()?;
        let field = self.logic_field(operands.field)?;
        let mode = self.batch_mode(operands.mode)?;
        let value = environment.batch_load_logic(prefab_hash, name_hash, field, mode)?;
        self.write(operands.destination, value)?;
        Ok(step_stop(environment))
    }

    fn execute_batch_store_logic<E: Ic10Environment>(
        &self,
        environment: &mut E,
        operands: BatchStoreOperands<'_>,
    ) -> Result<StepStop, Ic10Fault> {
        let prefab_hash = self.value(operands.prefab_hash)?;
        let name_hash = operands
            .name_hash
            .map(|operand| self.value(operand))
            .transpose()?;
        let field = self.logic_field(operands.field)?;
        let value = self.value(operands.value)?;
        environment.batch_store_logic(prefab_hash, name_hash, field, value)?;
        Ok(step_stop(environment))
    }

    fn execute_batch_load_slot_logic<E: Ic10Environment>(
        &mut self,
        environment: &mut E,
        operands: BatchSlotLoadOperands<'_>,
    ) -> Result<StepStop, Ic10Fault> {
        let prefab_hash = self.value(operands.prefab_hash)?;
        let name_hash = operands
            .name_hash
            .map(|operand| self.value(operand))
            .transpose()?;
        let slot = numeric_index(self.value(operands.slot)?)?;
        let field = self.logic_field(operands.field)?;
        let mode = self.batch_mode(operands.mode)?;
        let value = environment.batch_load_slot_logic(BatchSlotLoadRequest {
            prefab_hash,
            name_hash,
            slot,
            field,
            mode,
        })?;
        self.write(operands.destination, value)?;
        Ok(step_stop(environment))
    }

    fn execute_batch_store_slot_logic<E: Ic10Environment>(
        &self,
        environment: &mut E,
        operands: BatchSlotStoreOperands<'_>,
    ) -> Result<StepStop, Ic10Fault> {
        let prefab_hash = self.value(operands.prefab_hash)?;
        let slot = numeric_index(self.value(operands.slot)?)?;
        let field = self.logic_field(operands.field)?;
        let value = self.value(operands.value)?;
        environment.batch_store_slot_logic(prefab_hash, slot, field, value)?;
        Ok(step_stop(environment))
    }

    fn execute_store_logic<E: Ic10Environment>(
        &self,
        environment: &mut E,
        device: &DeviceOperand,
        field: &LogicFieldOperand,
        value: &ValueOperand,
    ) -> Result<StepStop, Ic10Fault> {
        let target = self.device_target(device)?;
        let field = self.logic_field(field)?;
        let value = self.value(value)?;
        environment.store_logic(target, field, value)?;
        Ok(step_stop(environment))
    }

    fn execute_load_slot_logic<E: Ic10Environment>(
        &mut self,
        environment: &mut E,
        operands: SlotLoadOperands<'_>,
    ) -> Result<StepStop, Ic10Fault> {
        let target = DeviceTarget::Port(self.device_port(operands.device)?);
        let slot = numeric_index(self.value(operands.slot)?)?;
        let field = self.logic_field(operands.field)?;
        let value = environment.load_slot_logic(target, slot, field)?;
        self.write(operands.destination, value)?;
        Ok(step_stop(environment))
    }

    fn execute_store_slot_logic<E: Ic10Environment>(
        &self,
        environment: &mut E,
        operands: SlotStoreOperands<'_>,
    ) -> Result<StepStop, Ic10Fault> {
        let target = DeviceTarget::Port(self.device_port(operands.device)?);
        let slot = numeric_index(self.value(operands.slot)?)?;
        let field = self.logic_field(operands.field)?;
        let value = self.value(operands.value)?;
        environment.store_slot_logic(target, slot, field, value)?;
        Ok(step_stop(environment))
    }

    fn execute_device_set<E: Ic10Environment>(
        &mut self,
        environment: &mut E,
        destination: RegisterRef,
        device: &DeviceOperand,
        expected_set: bool,
    ) -> Result<StepStop, Ic10Fault> {
        let target = self.device_target(device)?;
        let is_set = environment.device_is_set(target);
        self.write(destination, bool_to_number(is_set == expected_set))?;
        Ok(StepStop::Continue)
    }

    fn execute_get_stack<E: Ic10Environment>(
        &mut self,
        environment: &mut E,
        destination: RegisterRef,
        device: &DeviceOperand,
        address: &ValueOperand,
    ) -> Result<StepStop, Ic10Fault> {
        let target = self.device_target(device)?;
        let address = numeric_index(self.value(address)?)?;
        let value = environment.get_stack(target, address)?;
        self.write(destination, value)?;
        Ok(StepStop::Continue)
    }

    fn execute_put_stack<E: Ic10Environment>(
        &self,
        environment: &mut E,
        device: &DeviceOperand,
        address: &ValueOperand,
        value: &ValueOperand,
    ) -> Result<StepStop, Ic10Fault> {
        let target = self.device_target(device)?;
        let address = numeric_index(self.value(address)?)?;
        let value = self.value(value)?;
        environment.put_stack(target, address, value)?;
        Ok(StepStop::Continue)
    }

    fn value(&self, operand: &ValueOperand) -> Result<f64, Ic10Fault> {
        match operand {
            ValueOperand::Register(register) => Ok(self.registers.read(*register)?),
            ValueOperand::Number(value) => Ok(*value),
            ValueOperand::Symbol(symbol) => self
                .program
                .constant(symbol)
                .or_else(|| self.program.label(symbol).and_then(usize_to_f64))
                .ok_or_else(|| Ic10Fault::UnknownSymbol(symbol.clone())),
        }
    }

    fn logic_field<'a>(&self, operand: &'a LogicFieldOperand) -> Result<&'a str, Ic10Fault> {
        match operand {
            LogicFieldOperand::Named(field) => Ok(field),
            LogicFieldOperand::Dynamic(value) => {
                let value = self.value(value)?;
                logic_types::name_from_value(value).ok_or(Ic10Fault::UnknownLogicType(value))
            }
        }
    }

    fn batch_mode(&self, operand: &BatchModeOperand) -> Result<BatchMode, Ic10Fault> {
        match operand {
            BatchModeOperand::Direct(mode) => Ok(*mode),
            BatchModeOperand::Dynamic(value) => {
                let value = self.value(value)?;
                BatchMode::from_f64(value).ok_or(Ic10Fault::InvalidBatchMode(value))
            }
        }
    }

    fn write(&mut self, register: RegisterRef, value: f64) -> Result<(), Ic10Fault> {
        Ok(self.registers.write(register, value)?)
    }

    fn device_target(&self, operand: &DeviceOperand) -> Result<DeviceTarget, Ic10Fault> {
        match operand {
            DeviceOperand::Port(port) => Ok(DeviceTarget::Port(self.device_port(*port)?)),
            DeviceOperand::Reference(reference) => Ok(DeviceTarget::ReferenceId(reference_id(
                self.value(reference)?,
            )?)),
        }
    }

    fn device_port(&self, operand: DevicePortOperand) -> Result<DevicePort, Ic10Fault> {
        match operand {
            DevicePortOperand::Direct(port) => Ok(port),
            DevicePortOperand::Indirect(register) => {
                let index = self.registers.read(register)?;
                let index = u8::try_from(numeric_index(index)?)
                    .map_err(|_| Ic10Fault::InvalidDevicePortIndex(index))?;
                DevicePort::from_pin_index(index)
                    .ok_or_else(|| Ic10Fault::InvalidDevicePortIndex(f64::from(index)))
            }
        }
    }

    fn unary(operation: UnaryOperation, value: f64) -> Result<f64, Ic10Fault> {
        let result = match operation {
            UnaryOperation::Abs => value.abs(),
            UnaryOperation::Ceil => value.ceil(),
            UnaryOperation::Exp => value.exp(),
            UnaryOperation::Floor => value.floor(),
            UnaryOperation::Log => value.ln(),
            UnaryOperation::Round => value.round(),
            UnaryOperation::Sqrt => value.sqrt(),
            UnaryOperation::Trunc => value.trunc(),
            UnaryOperation::Acos => value.acos(),
            UnaryOperation::Asin => value.asin(),
            UnaryOperation::Atan => value.atan(),
            UnaryOperation::Cos => value.cos(),
            UnaryOperation::Sin => value.sin(),
            UnaryOperation::Tan => value.tan(),
            UnaryOperation::Not => i64_to_f64(!f64_to_i64(value)?)?,
            UnaryOperation::Seqz => bool_to_number(numeric_eq(value, 0.0)),
            UnaryOperation::Sgez => bool_to_number(value >= 0.0),
            UnaryOperation::Sgtz => bool_to_number(value > 0.0),
            UnaryOperation::Slez => bool_to_number(value <= 0.0),
            UnaryOperation::Sltz => bool_to_number(value < 0.0),
            UnaryOperation::Snan => bool_to_number(value.is_nan()),
            UnaryOperation::Snanz => bool_to_number(!value.is_nan()),
            UnaryOperation::Snez => bool_to_number(numeric_ne(value, 0.0)),
        };
        Ok(result)
    }

    fn binary(operation: BinaryOperation, left: f64, right: f64) -> Result<f64, Ic10Fault> {
        let result = match operation {
            BinaryOperation::Add => left + right,
            BinaryOperation::Sub => left - right,
            BinaryOperation::Mul => left * right,
            BinaryOperation::Div => left / right,
            BinaryOperation::Mod => ic10_mod(left, right),
            BinaryOperation::Pow => left.powf(right),
            BinaryOperation::Max => left.max(right),
            BinaryOperation::Min => left.min(right),
            BinaryOperation::Atan2 => left.atan2(right),
            BinaryOperation::And => i64_to_f64(f64_to_i64(left)? & f64_to_i64(right)?)?,
            BinaryOperation::Or => i64_to_f64(f64_to_i64(left)? | f64_to_i64(right)?)?,
            BinaryOperation::Xor => i64_to_f64(f64_to_i64(left)? ^ f64_to_i64(right)?)?,
            BinaryOperation::Nor => i64_to_f64(!(f64_to_i64(left)? | f64_to_i64(right)?))?,
            BinaryOperation::Sla | BinaryOperation::Sll => {
                i64_to_f64(f64_to_i64(left)?.wrapping_shl(shift_amount(right)?))?
            }
            BinaryOperation::Sra => {
                i64_to_f64(f64_to_i64(left)?.wrapping_shr(shift_amount(right)?))?
            }
            BinaryOperation::Srl => {
                let left_bits = u64::from_ne_bytes(f64_to_i64(left)?.to_ne_bytes());
                u64_to_f64(left_bits.wrapping_shr(shift_amount(right)?))?
            }
            BinaryOperation::Seq => bool_to_number(numeric_eq(left, right)),
            BinaryOperation::Sne => bool_to_number(numeric_ne(left, right)),
            BinaryOperation::Sge => bool_to_number(left >= right),
            BinaryOperation::Sgt => bool_to_number(left > right),
            BinaryOperation::Sle => bool_to_number(left <= right),
            BinaryOperation::Slt => bool_to_number(left < right),
            BinaryOperation::Sapz => bool_to_number(approximately_zero(left, right)),
            BinaryOperation::Snaz => {
                bool_to_number(!approximately_zero_without_epsilon_factor(left, right))
            }
        };
        Ok(result)
    }

    fn branch_condition<E: Ic10Environment>(
        &self,
        environment: &mut E,
        condition: &BranchCondition,
    ) -> Result<bool, Ic10Fault> {
        match condition {
            BranchCondition::Compare {
                operation,
                left,
                right,
            } => Ok(compare(*operation, self.value(left)?, self.value(right)?)),
            BranchCondition::CompareZero { operation, value } => {
                Ok(compare_zero(*operation, self.value(value)?))
            }
            BranchCondition::Approx {
                operation,
                left,
                right,
                tolerance,
            } => {
                let approximately = approximately(
                    self.value(left)?,
                    self.value(right)?,
                    self.value(tolerance)?,
                );
                Ok(match operation {
                    ApproxOperation::Approximately => approximately,
                    ApproxOperation::NotApproximately => !approximately,
                })
            }
            BranchCondition::ApproxZero {
                operation,
                value,
                tolerance,
            } => {
                let approximately = approximately_zero(self.value(value)?, self.value(tolerance)?);
                Ok(match operation {
                    ApproxZeroOperation::ApproximatelyZero => approximately,
                    ApproxZeroOperation::NotApproximatelyZero => !approximately,
                })
            }
            BranchCondition::Nan { value } => Ok(self.value(value)?.is_nan()),
            BranchCondition::DeviceSet {
                device,
                expected_set,
            } => {
                let target = self.device_target(device)?;
                Ok(environment.device_is_set(target) == *expected_set)
            }
            BranchCondition::DeviceValid {
                operation,
                device,
                field,
                expected_valid,
            } => {
                let target = self.device_target(device)?;
                let field = self.logic_field(field)?;
                let valid = match operation {
                    DeviceLogicOperation::Load => environment.can_load_logic(target, field),
                    DeviceLogicOperation::Store => environment.can_store_logic(target, field),
                };
                Ok(valid == *expected_valid)
            }
        }
    }

    fn jump(
        &mut self,
        target: &JumpTarget,
        link: bool,
        relative: bool,
        current_pc: usize,
    ) -> Result<(), Ic10Fault> {
        if link {
            let return_address = usize_to_f64(self.program_counter)
                .ok_or(Ic10Fault::ProgramCounterTooLarge(self.program_counter))?;
            self.write(RegisterRef::ReturnAddress, return_address)?;
        }

        let target = self.target_index(target, relative, current_pc)?;
        if target > self.program.len() {
            return Err(Ic10Fault::InvalidJumpTarget(target));
        }
        self.program_counter = target;
        Ok(())
    }

    fn target_index(
        &self,
        target: &JumpTarget,
        relative: bool,
        current_pc: usize,
    ) -> Result<usize, Ic10Fault> {
        match target {
            JumpTarget::Number(value) if relative => {
                let offset = f64_to_i64(*value)?;
                add_relative(current_pc, offset)
            }
            JumpTarget::Number(value) => numeric_index(*value),
            JumpTarget::Register(register) if relative => {
                let offset = f64_to_i64(self.registers.read(*register)?)?;
                add_relative(current_pc, offset)
            }
            JumpTarget::Register(register) => numeric_index(self.registers.read(*register)?),
            JumpTarget::Symbol(symbol) if relative => {
                let value = self
                    .program
                    .constant(symbol)
                    .ok_or_else(|| Ic10Fault::UnknownSymbol(symbol.clone()))?;
                let offset = f64_to_i64(value)?;
                add_relative(current_pc, offset)
            }
            JumpTarget::Symbol(symbol) => self
                .program
                .label(symbol)
                .or_else(|| {
                    self.program
                        .constant(symbol)
                        .and_then(|value| numeric_index(value).ok())
                })
                .ok_or_else(|| Ic10Fault::UnknownSymbol(symbol.clone())),
        }
    }

    fn next_random(&mut self) -> f64 {
        self.random_state = self
            .random_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let upper = self.random_state >> 11;
        #[allow(clippy::cast_precision_loss)]
        let numerator = upper as f64;
        #[allow(clippy::cast_precision_loss)]
        let denominator = (1_u64 << 53) as f64;
        numerator / denominator
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepStop {
    Continue,
    Disabled,
    Yielded,
    Halted,
}

#[derive(Clone, Copy)]
struct BatchLoadOperands<'a> {
    destination: RegisterRef,
    prefab_hash: &'a ValueOperand,
    name_hash: Option<&'a ValueOperand>,
    field: &'a LogicFieldOperand,
    mode: &'a BatchModeOperand,
}

#[derive(Clone, Copy)]
struct BatchStoreOperands<'a> {
    prefab_hash: &'a ValueOperand,
    name_hash: Option<&'a ValueOperand>,
    field: &'a LogicFieldOperand,
    value: &'a ValueOperand,
}

#[derive(Clone, Copy)]
struct BatchSlotLoadOperands<'a> {
    destination: RegisterRef,
    prefab_hash: &'a ValueOperand,
    name_hash: Option<&'a ValueOperand>,
    slot: &'a ValueOperand,
    field: &'a LogicFieldOperand,
    mode: &'a BatchModeOperand,
}

#[derive(Clone, Copy)]
struct BatchSlotStoreOperands<'a> {
    prefab_hash: &'a ValueOperand,
    slot: &'a ValueOperand,
    field: &'a LogicFieldOperand,
    value: &'a ValueOperand,
}

#[derive(Clone, Copy)]
struct SlotLoadOperands<'a> {
    destination: RegisterRef,
    device: DevicePortOperand,
    slot: &'a ValueOperand,
    field: &'a LogicFieldOperand,
}

#[derive(Clone, Copy)]
struct SlotStoreOperands<'a> {
    device: DevicePortOperand,
    slot: &'a ValueOperand,
    field: &'a LogicFieldOperand,
    value: &'a ValueOperand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RunStop {
    Yielded,
    Disabled,
    BudgetExhausted,
    Halted,
}

impl fmt::Display for RunStop {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yielded => formatter.write_str("yield"),
            Self::Disabled => formatter.write_str("disabled"),
            Self::BudgetExhausted => formatter.write_str("budget"),
            Self::Halted => formatter.write_str("halt"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RunResult {
    pub(super) instructions_executed: u32,
    pub(super) stop: RunStop,
}

#[derive(Debug)]
pub(super) enum Ic10Fault {
    UnknownSymbol(String),
    UnknownLogicType(f64),
    InvalidJumpTarget(usize),
    ProgramCounterTooLarge(usize),
    InvalidNumericIndex(f64),
    InvalidReferenceId(f64),
    InvalidDevicePortIndex(f64),
    InvalidBatchMode(f64),
    InvalidIntegerOperand(f64),
    InvalidShiftOperand(i64),
    InvalidBitFieldRange { offset: i64, length: i64 },
    RelativeJumpOutOfRange(i64),
    IntegerNotExactlyRepresentable(i64),
    UnsignedIntegerNotExactlyRepresentable(u64),
    Register(RegisterFault),
    Stack(StackFault),
    Environment(EnvironmentFault),
    HaltAndCatchFire { pc: usize },
}

impl fmt::Display for Ic10Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSymbol(symbol) => write!(formatter, "unknown symbol `{symbol}`"),
            Self::UnknownLogicType(value) => write!(formatter, "unknown logic type `{value}`"),
            Self::InvalidJumpTarget(target) => write!(formatter, "invalid jump target `{target}`"),
            Self::ProgramCounterTooLarge(pc) => {
                write!(formatter, "program counter too large: {pc}")
            }
            Self::InvalidNumericIndex(value) => {
                write!(formatter, "invalid numeric index `{value}`")
            }
            Self::InvalidReferenceId(value) => {
                write!(formatter, "invalid ReferenceId `{value}`")
            }
            Self::InvalidDevicePortIndex(value) => {
                write!(formatter, "invalid device port index `{value}`")
            }
            Self::InvalidBatchMode(value) => write!(formatter, "invalid batch mode `{value}`"),
            Self::InvalidIntegerOperand(value) => {
                write!(formatter, "expected integer operand, got `{value}`")
            }
            Self::InvalidShiftOperand(value) => {
                write!(formatter, "invalid shift operand `{value}`")
            }
            Self::InvalidBitFieldRange { offset, length } => {
                write!(
                    formatter,
                    "invalid bit field range offset={offset} length={length}"
                )
            }
            Self::RelativeJumpOutOfRange(value) => {
                write!(formatter, "relative jump offset out of range `{value}`")
            }
            Self::IntegerNotExactlyRepresentable(value) => {
                write!(
                    formatter,
                    "integer result is not exactly representable: {value}"
                )
            }
            Self::UnsignedIntegerNotExactlyRepresentable(value) => {
                write!(
                    formatter,
                    "unsigned integer result is not exactly representable: {value}"
                )
            }
            Self::Register(error) => write!(formatter, "{error}"),
            Self::Stack(error) => write!(formatter, "{error}"),
            Self::Environment(error) => write!(formatter, "{error}"),
            Self::HaltAndCatchFire { pc } => write!(formatter, "hcf executed at pc {pc}"),
        }
    }
}

impl From<RegisterFault> for Ic10Fault {
    fn from(value: RegisterFault) -> Self {
        Self::Register(value)
    }
}

impl From<StackFault> for Ic10Fault {
    fn from(value: StackFault) -> Self {
        Self::Stack(value)
    }
}

impl From<EnvironmentFault> for Ic10Fault {
    fn from(value: EnvironmentFault) -> Self {
        Self::Environment(value)
    }
}

fn step_stop<E: Ic10Environment>(environment: &E) -> StepStop {
    if environment.should_suspend_execution() {
        StepStop::Disabled
    } else {
        StepStop::Continue
    }
}

fn ternary(operation: TernaryOperation, first: f64, second: f64, third: f64) -> f64 {
    match operation {
        TernaryOperation::Lerp => (second - first).mul_add(third.clamp(0.0, 1.0), first),
        TernaryOperation::Sap => bool_to_number(approximately(first, second, third)),
        TernaryOperation::Sna => bool_to_number(!approximately(first, second, third)),
        TernaryOperation::Ext | TernaryOperation::Ins => unreachable!("handled before ternary"),
    }
}

fn bit_extract(source: f64, offset: f64, length: f64) -> Result<f64, Ic10Fault> {
    let (offset, length) = bit_field_range(offset, length)?;
    if length == 0 {
        return Ok(0.0);
    }
    let source = i64_bits(f64_to_i64(source)?);
    let value = (source >> offset) & bit_mask(length);
    u64_to_f64(value)
}

fn bit_insert(base: f64, field: f64, offset: f64, length: f64) -> Result<f64, Ic10Fault> {
    let (offset, length) = bit_field_range(offset, length)?;
    if length == 0 {
        return Ok(base);
    }
    let base = i64_bits(f64_to_i64(base)?);
    let field = i64_bits(f64_to_i64(field)?) & bit_mask(length);
    let shifted_mask = bit_mask(length) << offset;
    let inserted = (base & !shifted_mask) | (field << offset);
    i64_to_f64(i64::from_ne_bytes(inserted.to_ne_bytes()))
}

fn bit_field_range(offset: f64, length: f64) -> Result<(u32, u32), Ic10Fault> {
    let offset = f64_to_i64(offset)?;
    let length = f64_to_i64(length)?;
    if offset < 0 || !(0..=53).contains(&length) || offset >= 64 || offset + length > 64 {
        return Err(Ic10Fault::InvalidBitFieldRange { offset, length });
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok((offset as u32, length as u32))
}

const fn bit_mask(length: u32) -> u64 {
    if length == 64 {
        u64::MAX
    } else {
        (1_u64 << length) - 1
    }
}

const fn i64_bits(value: i64) -> u64 {
    u64::from_ne_bytes(value.to_ne_bytes())
}

fn compare(operation: CompareOperation, left: f64, right: f64) -> bool {
    match operation {
        CompareOperation::Eq => numeric_eq(left, right),
        CompareOperation::Ne => numeric_ne(left, right),
        CompareOperation::Ge => left >= right,
        CompareOperation::Gt => left > right,
        CompareOperation::Le => left <= right,
        CompareOperation::Lt => left < right,
    }
}

fn compare_zero(operation: CompareZeroOperation, value: f64) -> bool {
    match operation {
        CompareZeroOperation::Eq => numeric_eq(value, 0.0),
        CompareZeroOperation::Ne => numeric_ne(value, 0.0),
        CompareZeroOperation::Ge => value >= 0.0,
        CompareZeroOperation::Gt => value > 0.0,
        CompareZeroOperation::Le => value <= 0.0,
        CompareZeroOperation::Lt => value < 0.0,
    }
}

fn approximately(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= (tolerance * left.abs().max(right.abs())).max(f64::EPSILON * 8.0)
}

fn approximately_zero(value: f64, tolerance: f64) -> bool {
    value.abs() <= (tolerance * value.abs()).max(f64::EPSILON * 8.0)
}

fn approximately_zero_without_epsilon_factor(value: f64, tolerance: f64) -> bool {
    value.abs() <= (tolerance * value.abs()).max(f64::EPSILON)
}

fn ic10_mod(left: f64, right: f64) -> f64 {
    let divisor = right.abs();
    let remainder = left % divisor;
    if numeric_eq(remainder, 0.0) {
        return 0.0;
    }
    if left < 0.0 {
        remainder + divisor
    } else if right < 0.0 {
        2.0_f64.mul_add(-remainder, left)
    } else {
        remainder
    }
}

const fn bool_to_number(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

#[allow(clippy::float_cmp)]
fn numeric_eq(left: f64, right: f64) -> bool {
    left == right
}

#[allow(clippy::float_cmp)]
fn numeric_ne(left: f64, right: f64) -> bool {
    left != right
}

fn numeric_index(value: f64) -> Result<usize, Ic10Fault> {
    if !value.is_finite() || value.fract() != 0.0 || value < 0.0 {
        return Err(Ic10Fault::InvalidNumericIndex(value));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let index = value as usize;
    Ok(index)
}

fn reference_id(value: f64) -> Result<ReferenceId, Ic10Fault> {
    if !value.is_finite() || value.fract() != 0.0 || value < 0.0 {
        return Err(Ic10Fault::InvalidReferenceId(value));
    }
    if value > f64::from(u32::MAX) {
        return Err(Ic10Fault::InvalidReferenceId(value));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(ReferenceId::new(value as u32))
}

fn add_relative(program_counter: usize, offset: i64) -> Result<usize, Ic10Fault> {
    match offset.cmp(&0) {
        Ordering::Less => {
            let magnitude = usize::try_from(offset.unsigned_abs())
                .map_err(|_| Ic10Fault::RelativeJumpOutOfRange(offset))?;
            program_counter
                .checked_sub(magnitude)
                .ok_or(Ic10Fault::InvalidJumpTarget(0))
        }
        Ordering::Equal => Ok(program_counter),
        Ordering::Greater => {
            let magnitude =
                usize::try_from(offset).map_err(|_| Ic10Fault::RelativeJumpOutOfRange(offset))?;
            program_counter
                .checked_add(magnitude)
                .ok_or(Ic10Fault::ProgramCounterTooLarge(program_counter))
        }
    }
}

fn usize_to_f64(value: usize) -> Option<f64> {
    let value = u32::try_from(value).ok()?;
    Some(f64::from(value))
}

fn shift_amount(value: f64) -> Result<u32, Ic10Fault> {
    let value = f64_to_i64(value)?;
    u32::try_from(value).map_err(|_| Ic10Fault::InvalidShiftOperand(value))
}

fn f64_to_i64(value: f64) -> Result<i64, Ic10Fault> {
    const I64_MIN_AS_F64: f64 = -9_223_372_036_854_775_808.0;
    const I64_MAX_AS_F64: f64 = 9_223_372_036_854_775_807.0;

    if !value.is_finite() || value.fract() != 0.0 {
        return Err(Ic10Fault::InvalidIntegerOperand(value));
    }
    if !(I64_MIN_AS_F64..=I64_MAX_AS_F64).contains(&value) {
        return Err(Ic10Fault::InvalidIntegerOperand(value));
    }
    #[allow(clippy::cast_possible_truncation)]
    Ok(value as i64)
}

#[allow(clippy::cast_precision_loss, clippy::missing_const_for_fn)]
fn i64_to_f64(value: i64) -> Result<f64, Ic10Fault> {
    const MAX_EXACT_INTEGER: u64 = 9_007_199_254_740_992;
    if value.unsigned_abs() > MAX_EXACT_INTEGER {
        return Err(Ic10Fault::IntegerNotExactlyRepresentable(value));
    }
    Ok(value as f64)
}

#[allow(clippy::cast_precision_loss, clippy::missing_const_for_fn)]
fn u64_to_f64(value: u64) -> Result<f64, Ic10Fault> {
    const MAX_EXACT_INTEGER: u64 = 9_007_199_254_740_992;
    if value > MAX_EXACT_INTEGER {
        return Err(Ic10Fault::UnsignedIntegerNotExactlyRepresentable(value));
    }
    Ok(value as f64)
}
