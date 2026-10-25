pub mod about;
pub mod handler;
pub mod item;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuGroup {
    RadioWindowPosition,
    RadioMonitorSelector,
    RadioIndicatorIconTheme,
}
