#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderReason {
    #[default]
    Initial,
    Interaction,
    ReleaseSettle,
    Programmatic,
}
