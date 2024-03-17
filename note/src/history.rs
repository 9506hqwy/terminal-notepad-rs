use crate::buffer::Row;
use crate::cursor::Coordinates;

#[derive(Default)]
pub struct History<P: Coordinates> {
    entries: Vec<(P, Operation<P>)>,
}

impl<P: Coordinates> History<P> {
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn record(&mut self, cursor: P, op: Operation<P>) {
        self.entries.push((cursor, op));
    }

    pub fn rollback(&mut self) -> Option<(P, Operation<P>)> {
        self.entries.pop()
    }
}

// -----------------------------------------------------------------------------------------------

pub enum Operation<P: Coordinates> {
    Append(P),
    DeleteChar(P, char),
    DeleteChars(P, Row),
    DeleteRow(P, Row),
    InsertChar(P),
    InsertChars(P, usize),
    InsertRow(P),
    ShrinkRow(P, Row),
    SplitRow(P),
    SquashRow(P),
}
