pub mod about;
pub mod handler;
pub mod item;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuGroup {
    RadioWindowPosition,
    RadioMonitorSelector,
    RadioIndicatorIconTheme,
}
