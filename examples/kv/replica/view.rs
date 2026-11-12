use rumors::UnorderedMessages;

struct View {
    updates: UnorderedMessages<Update>,
    table: Table,
}
