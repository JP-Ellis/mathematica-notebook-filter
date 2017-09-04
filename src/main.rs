// mathematica-notebook-filter
// Copyright (C) 2017  Joshua Ellis <josh@jpellis.me>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
// details.
//
// You should have received a copy of the GNU General Public License along with
// this program.  If not, see <http://www.gnu.org/licenses/>.

use std::io;
use std::process::exit;
use std::error::Error;

mod parser;

fn main() {
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    exit(match parser::parse_input(
        &mut stdin_lock,
        &mut stdout_lock,
    ) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!(
                "Error when parsing notebook: {} ({:?})",
                e.description(),
                e.kind()
            );
            1
        }
    });
}
