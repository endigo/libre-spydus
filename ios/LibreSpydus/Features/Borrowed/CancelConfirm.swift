import SwiftUI

extension View {
    /// "Cancel this reservation?" anchored to `row`'s card, shown while `confirming` is that row.
    /// Cancelling is a real change at the library, so it only runs after the destructive button;
    /// anything else keeps the reservation.
    func cancelReservationConfirmation(for row: ReservationRow, confirming: Binding<ReservationRow?>,
                                       onConfirm: @escaping (ReservationRow) -> Void) -> some View {
        confirmationDialog(
            "Cancel this reservation?",
            isPresented: Binding(get: { confirming.wrappedValue?.id == row.id },
                                 set: { if !$0, confirming.wrappedValue?.id == row.id { confirming.wrappedValue = nil } }),
            titleVisibility: .visible
        ) {
            Button("Cancel Reservation", role: .destructive) { onConfirm(row) }
            Button("Keep Reservation", role: .cancel) {}
        } message: {
            Text(CancelConfirm.message(for: row))
        }
    }
}

enum CancelConfirm {
    static func message(for row: ReservationRow) -> String {
        if row.isReady {
            let place = row.pickup.map { " at \($0)" } ?? ""
            return "“\(row.ref.title)” is waiting for you\(place). Cancelling releases it to the next person."
        }
        return "“\(row.ref.title)” will be removed from your reservations and you'll lose your place in the queue."
    }
}
