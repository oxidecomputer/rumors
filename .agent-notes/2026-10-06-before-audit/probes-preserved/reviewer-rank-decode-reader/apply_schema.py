import pathlib, sys
R = pathlib.Path(sys.argv[0]).resolve().parent
rank = pathlib.Path("/Users/oxide/src/rumors-slot-35/crates/before/src/rank.rs")
text = rank.read_text()
start = text.index("    pub fn decode<R: Read>(mut reader: R) -> Result<Rank, Decode> {\n")
stop = text.index("    /// Decodes canonical bytes already held in memory.\n")
assert text.count("    pub fn decode<R: Read>(mut reader: R)") == 1
rank.write_text(text[:start] + (R / "decode_schema.rs").read_text() + text[stop:])
print("applied")
