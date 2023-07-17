pub trait CommandManager: Send + Sync + 'static {
    type Command: Command;
}

pub trait Command: Send + Sync + 'static {}

