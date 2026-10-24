/// The last response the Requests section shows. `requestPath` is nil for an ad-hoc send.
struct ShownResponse {
    let requestPath: String?
    let exchange: SentExchange
    let body: PreparedBody
}
