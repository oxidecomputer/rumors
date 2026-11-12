impl Replica {
    fn into_rumors(self) -> Rumors<Update, FileBookmark<TokioBlocking>> {
        self.rumors
    }
}
