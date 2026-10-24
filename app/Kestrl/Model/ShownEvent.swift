/// One history event, with its bodies already prepared for display.
struct ShownEvent {
    let detail: EventDetail
    let requestBody: PreparedBody?
    let responseBody: PreparedBody?
}
