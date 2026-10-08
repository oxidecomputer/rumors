    pub fn decode<R: Read>(mut reader: R) -> Result<Rank, Decode> {
        // REVIEWER MUTANT SCHEMA (temporary): RANK_MUTANT selects one mutant.
        let m: u32 = std::env::var("RANK_MUTANT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let guard = move |t: u32, error: &io::Error| -> bool {
            if m == t {
                true
            } else if m == t + 1 {
                false
            } else if m == t + 2 {
                error.kind() != io::ErrorKind::Interrupted
            } else {
                error.kind() == io::ErrorKind::Interrupted
            }
        };
        let mut buf = [0; DECODE_CHUNK_BYTES];
        let mut end = 0;
        let at_eof = loop {
            match reader.read(&mut buf[end..]) {
                Ok(0) => break true,
                Ok(read) => {
                    end += read;
                    if m == 56 {
                        if let Err(e @ (Decode::TrailingBits | Decode::NotCanonical)) =
                            Self::decode_bytes(&buf[..end])
                        {
                            return Err(e);
                        }
                    }
                    if end == buf.len() {
                        break false;
                    }
                }
                Err(error) if guard(10, &error) => {}
                Err(error) => {
                    if m == 50 {
                        break true;
                    }
                    if m == 51 {
                        if let Ok(rank) = Self::decode_bytes(&buf[..end]) {
                            return Ok(rank);
                        }
                    }
                    if m == 55 {
                        if let Err(e @ (Decode::TrailingBits | Decode::NotCanonical)) =
                            Self::decode_bytes(&buf[..end])
                        {
                            return Err(e);
                        }
                    }
                    return Err(Decode::Io(error));
                }
            }
        };

        match Self::decode_bytes(&buf[..end]) {
            Ok(rank) if at_eof || m == 2 => return Ok(rank),
            Ok(rank) => loop {
                match reader.read(&mut buf[..1]) {
                    Ok(0) => return Ok(rank),
                    Ok(_) => return Err(Decode::TrailingBits),
                    Err(error) if guard(20, &error) => {}
                    Err(error) => {
                        if m == 52 {
                            return Ok(rank);
                        }
                        return Err(Decode::Io(error));
                    }
                }
            },
            Err(Decode::Truncated) if at_eof => return Err(Decode::Truncated),
            Err(Decode::Truncated) => {}
            Err(error) => return Err(error),
        }

        let mut next = 0;
        let rank = Self::decode_stream(|| loop {
            if next < end {
                let byte = buf[next];
                next += 1;
                return Ok(byte);
            }
            match reader.read(&mut buf) {
                Ok(0) => return Err(Decode::Truncated),
                Ok(read) => {
                    next = 0;
                    end = read;
                }
                Err(error) if guard(30, &error) => {}
                Err(error) => {
                    if m == 54 {
                        return Err(Decode::Truncated);
                    }
                    return Err(Decode::Io(error));
                }
            }
        })?;

        if next < end {
            return Err(Decode::TrailingBits);
        }
        loop {
            match reader.read(&mut buf[..1]) {
                Ok(0) => return Ok(rank),
                Ok(_) => return Err(Decode::TrailingBits),
                Err(error) if guard(40, &error) => {}
                Err(error) => {
                    if m == 53 {
                        return Ok(rank);
                    }
                    return Err(Decode::Io(error));
                }
            }
        }
    }

