import SwiftUI

/// Book detail for `Route.detail` (I4): navy header with cover and "Borrow Now", white sheet
/// with synopsis, subjects, details and related carousels. Wave-1 stub.
struct BookDetailView: View {
    let ref: BookRef

    var body: some View {
        PlaceholderScreen(title: ref.title.isEmpty ? "Book" : ref.title, systemImage: "book.closed",
                          message: Formatting.byline(ref.author) ?? ref.url)
            .navigationBarTitleDisplayMode(.inline)
    }
}
