#[derive(Clone, Copy)]
pub enum DinoCommand {
    JumpPressed,
    JumpReleased,
    DuckPressed,
    DuckReleased,
    Restart,
    TogglePause,
}
