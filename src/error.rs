#[derive(Debug, PartialEq, Eq)]
pub enum ErrorKind {
	Parse,
	IdentifierUndefined,
	ObjectNotIterable,
	Internal
}

#[derive(Debug, PartialEq, Eq)]
pub struct TemplateError {
	pub kind: ErrorKind,
    pub message : String
}