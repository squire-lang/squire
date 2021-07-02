use super::{Error, Compilable};
use std::collections::{HashMap, hash_map::Entry};
use std::rc::Rc;
use std::cell::RefCell;

use crate::runtime::{Bytecode, Opcode, Interrupt, CodeBlock, Vm};
use crate::value::Value;
use crate::parse::{Parsable, Parser};

#[derive(Debug)]
struct Label {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Target(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Constant(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Global(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JumpDestination(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodePosition(usize);

pub type Globals = Rc<RefCell<HashMap<String, (Global, Option<Value>)>>>;

#[derive(Debug)]
pub struct Compiler {
	globals: Globals,
	code: Vec<Bytecode>,
	ntargets: usize,
	labels: Vec<Label>,
	locals: HashMap<String, Target>,
	constants: Vec<Value>,
}

impl Default for Compiler {
	fn default() -> Self {
		let mut globals = HashMap::new();

		for default in crate::value::builtin::defaults() {
			globals.insert(default.name().to_string(), (Global(globals.len()), Some(Value::BuiltinJourney(default))));
		}

		Self {
			globals: Rc::new(RefCell::new(globals)),
			code: Default::default(),
			ntargets: 1, // as `0` is always reserved for temp
			labels: Default::default(),
			locals: Default::default(),
			constants: Default::default(),
		}
	}
}

impl Compiler {
	pub fn with_globals(globals: Globals) -> Self {
		Self { globals, ..Self::default() }
	}

	pub fn compile_with<I: Iterator<Item=char>>(&mut self, parser: &mut Parser<I>) -> Result<(), Error> {
		while let Some(statement) = crate::ast::Statement::parse(parser)? {
			statement.compile(self, None)?;
		}

		Ok(())
	}

	pub fn convert_globals(&self) -> Vec<Value> {
		let globals = self.globals.borrow();
		let mut globals_vec = vec![Value::Ni; globals.len()];

		for (Global(index), value) in globals.values() {
			globals_vec[*index] = (*value).as_ref().cloned().unwrap_or_default();
		}

		globals_vec
	}

	pub fn globals(&self) -> &Globals {
		&self.globals
	}

	pub fn finish(self) -> CodeBlock {
		trace!(?self);
		CodeBlock::new(self.ntargets, self.code, self.constants)
	}

	pub fn finish_with_vm(self) -> (CodeBlock, Vm) {
		let vm = Vm::new(self.convert_globals());
		(self.finish(), vm)
	}

	fn bytecode(&mut self, bytecode: Bytecode) {
		trace!(codelen=%self.code.len(), ?bytecode);
		self.code.push(bytecode);
	}

	pub fn opcode(&mut self, opcode: Opcode) {
		self.bytecode(Bytecode::Opcode(opcode));
	}

	pub fn count(&mut self, amount: usize) {
		self.bytecode(Bytecode::Count(amount));
	}

	pub fn interrupt(&mut self, interrupt: Interrupt) {
		self.bytecode(Bytecode::Interrupt(interrupt));
	}

	pub fn target(&mut self, target: Target) {
		debug_assert!(target.0 <= self.ntargets);

		self.bytecode(Bytecode::Local(target.0));
	}

	pub fn constant(&mut self, constant: Constant) {
		debug_assert!(constant.0 < self.constants.len());

		self.bytecode(Bytecode::Constant(constant.0));
	}

	pub fn global(&mut self, global: Global) {
		debug_assert!(global.0 < self.globals.borrow().len());

		self.bytecode(Bytecode::Global(global.0));
	}

	pub fn defer_jump(&mut self) -> JumpDestination {
		self.bytecode(Bytecode::Illegal);

		JumpDestination(self.code.len() - 1)
	}

	pub fn jump_to(&mut self, dst: CodePosition) {
		self.defer_jump().set_jump_to(dst, self);
	}

	pub fn get_constant(&mut self, constant: Value) -> Constant {
		if let Some(index) = self.constants.iter().position(|x| *x == constant) {
			Constant(index)
		} else {
			self.constants.push(constant);
			Constant(self.constants.len() - 1)
		}
	}

	pub fn get_local(&mut self, name: &str) -> Option<Target> {
		self.locals.get(name).cloned()
	}

	pub fn define_local(&mut self, name: String) -> Target {
		if let Some(&target) = self.locals.get(&name) {
			target
		} else {
			let target = self.next_target();
			self.locals.insert(name, target);
			target
		}
	}

	pub fn get_global(&mut self, name: &str) -> Option<(Global, Option<Value>)> {
		self.globals.borrow().get(name).cloned()
	}

	pub fn define_global(&mut self, name: String, value: Option<Value>) -> Result<Global, Error> {
		let mut globals = self.globals.borrow_mut();

		let global = Global(globals.len());
		let entry = globals.entry(name);

		match (entry, value) {
			(Entry::Occupied(occupied), Some(_)) if occupied.get().1.is_some()
				=> Err(Error::GlobalAlreadyDefined(occupied.key().to_string())),

			(Entry::Occupied(mut occupied), value) => {
				occupied.get_mut().1 = value;
				Ok(occupied.get().0)
			},
			(Entry::Vacant(vacant), value) => {
				vacant.insert((global, value));
				Ok(global)
			}
		}
	}

	pub fn next_target(&mut self) -> Target {
		self.ntargets += 1;
		Target(self.ntargets - 1)
	}

	pub const SCRATCH_TARGET: Target = Target(0);

	pub const fn temp_target(&self) -> Target {
		Self::SCRATCH_TARGET
	}

	pub fn current_pos(&self) -> CodePosition {
		CodePosition(self.code.len())
	}
}

impl JumpDestination {
	pub fn set_jump_to_current(self, compiler: &mut Compiler) {
		self.set_jump_to(CodePosition(compiler.code.len()), compiler)
	}

	pub fn set_jump_to(self, pos: CodePosition, compiler: &mut Compiler) {
		let relative = pos.0 as isize - (self.0 as isize);

		trace!(at=%self.0, %relative, absolute=%pos.0, "updated jump dst");

		debug_assert_eq!(compiler.code[self.0], Bytecode::Illegal, "bad byteode at {:?}", self.0);
		compiler.code[self.0] = Bytecode::Offset(relative);
	}
}