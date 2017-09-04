use std::io;

mod header;
mod notebook;
mod cell;
mod cell_group_data;
mod utilities;

pub use self::header::{parse_content_type, parse_beginning_notebook};
pub use self::notebook::parse_notebook;
pub use self::cell::{parse_cell_list, parse_cell};
pub use self::cell_group_data::parse_cell_group_data;

pub fn parse_input<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    parse_content_type(input, output)
        .and(parse_beginning_notebook(input, output))
        .and(parse_notebook(input, output))
}
