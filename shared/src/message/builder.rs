pub trait MessageBuilder {
    type Message;
    type MessageKind;

    fn kind(self, kind: Self::MessageKind) -> Self;
    fn build(self) -> Self::Message;
}
