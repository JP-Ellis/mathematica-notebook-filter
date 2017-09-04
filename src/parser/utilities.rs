use std::io;

/// Parse a function of the form `Foo[...]` from the buffer, and return a slice
/// containing *only* the function (or an error if the input ended before the
/// end of the function).  It returns the pair `s: Vec<u8>` and `args:
/// Vec<usize>`.  The first is the list of bytes that make up the full function.
/// The second is a list of indices where the arguments start and end, such that
/// argument `n` starts at index `args[n] + 1` and ends at `args[n+1]`.  The
/// `+1` offset is due to the comma (or opening `[`).
///
/// The function recognized strings, but does **not** recognize comments as
/// Mathematica does not produce any comments within the main `Notebook[]`
/// function itself.
pub fn load_function<I>(input: &mut I) -> Result<(Vec<u8>, Vec<usize>), io::Error>
where
    I: io::BufRead,
{
    let mut s = Vec::new();
    let mut args = Vec::new();

    let mut depth = 0;
    let mut in_string = false;
    let mut idx = 0;
    while depth > 0 || args.len() == 0 {
        let consumed_length = {
            let buf = input.fill_buf()?;

            if buf.len() == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "EOF reached before end of a function.",
                ));
            }

            let mut buf_idx = 0;
            for &c in buf {
                // Check if we've gone passed the end of the function; and if
                // not, add the byte to the output.
                if depth == 0 && args.len() > 0 {
                    break;
                } else {
                    s.push(c);
                    idx += 1;
                    buf_idx += 1;
                }

                match (c, depth, in_string) {
                    (b'"', _, _) => in_string = !in_string,

                    (b',', 1, false) => args.push(idx),
                    (b',', _, false) => {}

                    (b'[', 0, false) => {
                        depth += 1;
                        args.push(idx);
                    }
                    (b'[', _, false) => depth += 1,

                    (b']', 1, false) => {
                        depth -= 1;
                        args.push(idx);
                    }
                    (b']', _, false) => depth -= 1,

                    (b'{', _, false) => depth += 1,
                    (b'}', _, false) => depth -= 1,

                    _ => {}
                }
            }

            buf_idx
        };

        input.consume(consumed_length);
    }

    Ok((s, args))
}

/// Parse the remainder of function of the form `Foo[...]` from the buffer, with
/// the buffer starting at any point within the square brackets, and return a
/// slice containing *only* the remainder of the (or an error if the input ended
/// before the end of the function).
///
/// It returns the pair `s: Vec<u8>` and `args: Vec<usize>`.  The first is the
/// list of bytes that make up the full function.  The second is a list of
/// indices where the arguments start and end, such that argument `n` starts at
/// index `args[n] + 1` and ends at `args[n+1]`.  The `+1` offset is due to the
/// comma (or closing `]`).
///
/// Note that since it is assumed that the function starts *inside* the
/// function, it does not know which argument it is up to.  Furthermore,
/// `args[0]` will indicate only where the next argument starts from where it
/// started parsing.
///
/// The function recognized strings, but does **not** recognize comments as
/// Mathematica does not produce any comments within the main `Notebook[]`
/// function itself.
pub fn load_rest_of_function<I>(input: &mut I) -> Result<(Vec<u8>, Vec<usize>), io::Error>
where
    I: io::BufRead,
{
    let mut s = Vec::new();
    let mut args = Vec::new();

    let mut depth = 1;
    let mut in_string = false;
    let mut idx = 0;
    while depth > 0 {
        let consumed_length = {
            let buf = input.fill_buf()?;

            if buf.len() == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "EOF reached before end of a function.",
                ));
            }

            let mut buf_idx = 0;
            for &c in buf {
                // Check if we've gone passed the end of the function; and if
                // not, add the byte to the output.
                if depth == 0 && args.len() > 0 {
                    break;
                } else {
                    s.push(c);
                    idx += 1;
                    buf_idx += 1;
                }

                match (c, depth) {
                    (b'"', _) => in_string = !in_string,

                    (b',', 1) => args.push(idx),
                    (b',', _) => {}

                    (b'[', 0) => {
                        depth += 1;
                        args.push(idx);
                    }
                    (b'[', _) => depth += 1,

                    (b']', 1) => {
                        depth -= 1;
                        args.push(idx);
                    }
                    (b']', _) => depth -= 1,

                    (b'{', _) => depth += 1,
                    (b'}', _) => depth -= 1,

                    _ => {}
                }
            }

            buf_idx
        };

        input.consume(consumed_length);
    }

    Ok((s, args))
}

#[cfg(test)]
mod test {
    #[test]
    fn load_function() {
        // Simple function
        ////////////////////////////////////////
        let mut input = &b"Foo[x, y, z] Bar[x, y, z]"[..];
        let (s, args) = super::load_function(&mut input).unwrap();
        assert_eq!(input, &b" Bar[x, y, z]"[..]);
        assert_eq!(s, &b"Foo[x, y, z]"[..]);
        assert_eq!(args, vec![4, 6, 9, 12]);
        assert_eq!(&s[args[0]..args[1]], &b"x,"[..]);
        assert_eq!(&s[args[1]..args[2]], &b" y,"[..]);
        assert_eq!(&s[args[2]..args[3]], &b" z]"[..]);

        // Nested functions
        ////////////////////////////////////////
        let mut input = &b"Foo[Bar[Sin[x, y], z], P[x]]"[..];
        let (s, args) = super::load_function(&mut input).unwrap();
        assert!(input.is_empty());
        assert_eq!(s, &b"Foo[Bar[Sin[x, y], z], P[x]]"[..]);
        assert_eq!(args, vec![4, 22, 28]);
        assert_eq!(&s[args[0]..args[1]], &b"Bar[Sin[x, y], z],"[..]);
        assert_eq!(&s[args[1]..args[2]], &b" P[x]]"[..]);

        // With list
        ////////////////////////////////////////
        let mut input = &b"Foo[{x, y, z}, {1, 2, 3}]"[..];
        let (s, args) = super::load_function(&mut input).unwrap();
        assert!(input.is_empty());
        assert_eq!(s, &b"Foo[{x, y, z}, {1, 2, 3}]"[..]);
        assert_eq!(args, vec![4, 14, 25]);
        assert_eq!(&s[args[0]..args[1]], &b"{x, y, z},"[..]);
        assert_eq!(&s[args[1]..args[2]], &b" {1, 2, 3}]"[..]);
    }

    #[test]
    fn load_rest_of_function() {
        // Simple function
        ////////////////////////////////////////
        let mut input = &b"x, y, z] Bar[x, y, z]"[..];
        let (s, args) = super::load_rest_of_function(&mut input).unwrap();
        assert_eq!(input, &b" Bar[x, y, z]"[..]);
        assert_eq!(s, &b"x, y, z]"[..]);
        assert_eq!(args, vec![2, 5, 8]);
        assert_eq!(&s[..args[0]], &b"x,"[..]);
        assert_eq!(&s[args[0]..args[1]], &b" y,"[..]);
        assert_eq!(&s[args[1]..args[2]], &b" z]"[..]);

        // Nested functions
        ////////////////////////////////////////
        let mut input = &b"Bar[Sin[x, y], z], P[x]]"[..];
        let (s, args) = super::load_rest_of_function(&mut input).unwrap();
        assert!(input.is_empty());
        assert_eq!(s, &b"Bar[Sin[x, y], z], P[x]]"[..]);
        assert_eq!(args, vec![18, 24]);
        assert_eq!(&s[..args[0]], &b"Bar[Sin[x, y], z],"[..]);
        assert_eq!(&s[args[0]..args[1]], &b" P[x]]"[..]);

        // With list
        ////////////////////////////////////////
        let mut input = &b"{x, y, z}, {1, 2, 3}]"[..];
        let (s, args) = super::load_rest_of_function(&mut input).unwrap();
        assert!(input.is_empty());
        assert_eq!(s, &b"{x, y, z}, {1, 2, 3}]"[..]);
        assert_eq!(args, vec![10, 21]);
        assert_eq!(&s[..args[0]], &b"{x, y, z},"[..]);
        assert_eq!(&s[args[0]..args[1]], &b" {1, 2, 3}]"[..]);
    }
}
