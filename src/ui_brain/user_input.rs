use crate::ui_brain::State;

#[derive(std::cmp::PartialEq)]
pub enum ExitType {
    Sucess,
    Aborted,
    Error,
}
impl ExitType {
    pub fn get_header(&self) -> &'static str {
        match self {
            Self::Aborted => "Aborted Operation",
            Self::Sucess => "Sucessful Operation",
            Self::Error => "Operation Failure",
        }
    }
}

pub struct UserOperationResult {
    pub exit: ExitType,
    pub message: Option<String>,
}

impl UserOperationResult {
    pub fn invalidate(&mut self, message: String) {
        self.exit = ExitType::Error;
        self.message = Some(message);
    }

    pub fn operation_on_prev() -> Self {
        UserOperationResult {
            exit: ExitType::Error,
            message: Some(String::from("Cannot operate on '..'")),
        }
    }
}

// returns how long to wait before reseting the state machine to selecting
type ClosureT = Box<dyn FnOnce(&mut State, &str) -> UserOperationResult>;
pub struct UserInputRequest {
    pub title: String,
    pub on_enter: Option<ClosureT>,
    pub edit_ressource: bool,
}

impl UserInputRequest {
    pub fn new(title: String, edit_ressource: bool, on_enter: ClosureT) -> UserInputRequest {
        UserInputRequest {
            title,
            on_enter: Some(on_enter),
            edit_ressource,
        }
    }
    pub fn get_closure(&mut self) -> ClosureT {
        self.on_enter.take().unwrap()
    }
}
