use crate::compile::Error as CompilerError;
use super::{Form, FormInner};
use std::sync::Arc;
use crate::value::journey::UserDefined;

pub struct FormBuilder(pub(super) FormInner);

impl FormBuilder {
	fn form_key_exists(&self, key: &str) -> bool {
		self.0.recalls.contains_key(key) || self.0.essences.contains_key(key)
	}

	fn imitation_key_exists(&self, key: &str) -> bool {
		self.0.changes.contains_key(key) || self.0.matter_names.iter().any(|x| x == key)
	}

	pub fn add_parent(&mut self, parent: Form) {
		// todo: do we want to check for the parent already existing?
		self.0.parents.push(parent);
	}

	pub fn add_recall(&mut self, recall: UserDefined) -> Result<(), CompilerError> {
		let name = recall.name().to_string();
		if self.form_key_exists(&name) {
			Err(CompilerError::FormValueAlreadyDefined { name, kind: "recall" })
		} else {
			self.0.recalls.insert(name.into(), recall.into());
			Ok(())
		}
	}

	pub fn add_essence(&mut self, name: String) -> Result<(), CompilerError> {
		if self.form_key_exists(&name) {
			Err(CompilerError::FormValueAlreadyDefined { name, kind: "essence" })
		} else {
			self.0.essences.insert(name, Default::default());
			Ok(())
		}
	}

	pub fn add_matter(&mut self, name: String) -> Result<(), CompilerError> {
		if self.imitation_key_exists(&name) {
			Err(CompilerError::FormValueAlreadyDefined { name, kind: "matter" })
		} else {
			self.0.matter_names.push(name);
			Ok(())
		}
	}

	pub fn add_change(&mut self, change: UserDefined) -> Result<(), CompilerError> {
		let name = change.name().to_string();
		if self.imitation_key_exists(&name) {
			Err(CompilerError::FormValueAlreadyDefined { name, kind: "change" })
		} else {
			self.0.changes.insert(name.into(), change.into());
			Ok(())
		}
	}

	pub fn add_imitate(&mut self, imitate: UserDefined) -> Result<(), CompilerError> {
		if self.0.imitate.is_some() {
			Err(CompilerError::FormValueAlreadyDefined { name: "imitate".to_string(), kind: "imitate" })
		} else {
			self.0.imitate = Some(imitate.into());
			Ok(())
		}
	}

	pub fn build(self) -> Form {
		Form(Arc::new(self.0))
	}
}
