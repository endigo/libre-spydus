import SwiftUI

extension View {
    /// "Cancel this reservation?" anchored to `row`'s card, shown while `confirming` is that row.
    /// Cancelling is a real change at the library, so it only runs after the destructive button;
    /// anything else keeps the reservation.
    func cancelReservationConfirmation(for row: ReservationRow, confirming: Binding<ReservationRow?>,
                                       onConfirm: @escaping (ReservationRow) -> Void) -> some View {
        dialogs {
            Color.clear.confirmationDialog(
                Copy.YourBooks.cancelTitle,
                isPresented: Binding(get: { confirming.wrappedValue?.id == row.id },
                                     set: { if !$0, confirming.wrappedValue?.id == row.id { confirming.wrappedValue = nil } }),
                titleVisibility: .visible
            ) {
                Button(Copy.YourBooks.cancelReservation, role: .destructive) { onConfirm(row) }
                Button(Copy.YourBooks.keepReservation, role: .cancel) {}
            } message: {
                Text(CancelConfirm.message(for: row))
            }
        }
    }
}

enum CancelConfirm {
    static func message(for row: ReservationRow) -> String {
        if row.isReady {
            guard let branch = row.pickup else { return Copy.YourBooks.cancelReadyMessageNoBranch(title: row.ref.title) }
            return Copy.YourBooks.cancelReadyMessage(title: row.ref.title, branch: branch)
        }
        return Copy.YourBooks.cancelPendingMessage(title: row.ref.title)
    }
}
