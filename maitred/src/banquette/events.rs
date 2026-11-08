use canner::{Tin, encrypted::Encrypted};

use crate::banquette::log::Head;

pub enum Write {
    AppendLog(Encrypted),
    DeleteLog(Head),
    EditLog(Tin, Encrypted),
    RewindLog(Head),
    UpdateState(String, Tin),
}

pub enum Read {
    ReadBiblio,
    CatchUp(Head),
    ReadState(String),
}

pub enum Events {
    Mut(Write),
    Imut(Read),
}
