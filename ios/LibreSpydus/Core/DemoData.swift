import Foundation
import LibreSpydusCore

/// The account fixtures `DemoBackend` serves, decoded from `Resources/DemoData/demo-*.json`.
///
/// Those files are the synthetic goldens (`core/tests/golden/synthetic/account-links.json`,
/// `patron-items.json`, `reserve-options.json`, `edit-options.json`; fabricated "Test Borrower")
/// plus loans/pending reservations built from the public `related.json` golden. Loans carry a
/// `dueInDays` offset; the "Due" field is generated relative to today. Every loan and reservation
/// links to its bib the way the OPAC does (`bibIrn` + a session-set `bibUrl`), and `catalogue`
/// holds those bibs' public records, so their book pages work offline.
struct DemoFixtures: Sendable {
    /// The origin every URL in the bundled JSON uses; rewritten to the demo backend's origin.
    static let fixtureOrigin = "https://canning.spydus.com"

    var account: AccountLinks
    var loans: [PatronItem]
    var ready: [PatronItem]
    var pending: [PatronItem]
    var reserveOptions: ReserveOptions
    var editOptions: EditOptions
    var catalogue: DemoCatalogue

    /// Loads the bundled fixtures, with URLs moved to `origin` and loan due dates set from `today`.
    static func bundled(origin: String = fixtureOrigin, today: Date = .now, bundle: Bundle = .main) throws -> DemoFixtures {
        func load<T: Decodable>(_ name: String, as type: T.Type) throws -> T {
            guard let url = bundle.url(forResource: name, withExtension: "json") else {
                throw CocoaError(.fileNoSuchFile, userInfo: [NSFilePathErrorKey: "\(name).json"])
            }
            var text = try String(contentsOf: url, encoding: .utf8)
            if origin != fixtureOrigin {
                text = text.replacingOccurrences(of: fixtureOrigin, with: origin)
            }
            return try JSONDecoder().decode(T.self, from: Data(text.utf8))
        }

        let reservations = try load("demo-reservations", as: ReservationsDTO.self)
        let loans = try load("demo-loans", as: LoansDTO.self).loans.map { loan in
            var item = loan.item.model
            let due = Calendar.current.date(byAdding: .day, value: loan.dueInDays, to: today) ?? today
            item.fields["Due"] = DueDate.format(due)
            return item
        }
        return DemoFixtures(
            account: try load("demo-account", as: AccountLinksDTO.self).model,
            loans: loans,
            ready: reservations.ready.items.map(\.model),
            pending: reservations.pending.items.map(\.model),
            reserveOptions: try load("demo-reserve-options", as: ReserveOptionsDTO.self).model,
            editOptions: try load("demo-edit-options", as: EditOptionsDTO.self).model,
            catalogue: DemoCatalogue(try load("demo-catalogue", as: DemoCatalogueDTO.self), origin: origin)
        )
    }
}

// MARK: - JSON shapes (the core's camelCase records; UniFFI types aren't Codable)

struct AccountLinksDTO: Decodable {
    var name: String?
    var loansUrl: String?
    var reservationsReadyUrl: String?
    var reservationsPendingUrl: String?

    var model: AccountLinks {
        AccountLinks(name: name, loansUrl: loansUrl, reservationsReadyUrl: reservationsReadyUrl,
                     reservationsPendingUrl: reservationsPendingUrl)
    }
}

struct PatronActionDTO: Decodable {
    var label: String
    var href: String
}

struct PatronItemDTO: Decodable {
    var irn: String?
    var bibIrn: String?
    var title: String?
    var bibUrl: String?
    var author: String?
    var year: String?
    var coverUrls: [String]
    var isbn: String?
    var fields: [String: String]
    var actions: [PatronActionDTO]

    var model: PatronItem {
        PatronItem(irn: irn, bibIrn: bibIrn, title: title, bibUrl: bibUrl, author: author, year: year,
                   coverUrls: coverUrls, isbn: isbn, fields: fields,
                   actions: actions.map { PatronAction(label: $0.label, href: $0.href) })
    }
}

struct PatronListDTO: Decodable {
    var total: Int64?
    var items: [PatronItemDTO]
}

struct ReservationsDTO: Decodable {
    var ready: PatronListDTO
    var pending: PatronListDTO
}

struct LoansDTO: Decodable {
    struct Loan: Decodable {
        var dueInDays: Int
        var item: PatronItemDTO
    }
    var loans: [Loan]
}

struct ReserveOptionsDTO: Decodable {
    struct Expiry: Decodable {
        var days: String
        var label: String
    }
    var canReserve: Bool
    var pickupLocation: String?
    var availableSummary: String?
    var expiryChoices: [Expiry]
    var message: String?

    var model: ReserveOptions {
        ReserveOptions(canReserve: canReserve, pickupLocation: pickupLocation, availableSummary: availableSummary,
                       expiryChoices: expiryChoices.map { ExpiryChoice(days: $0.days, label: $0.label) },
                       message: message)
    }
}

struct EditOptionsDTO: Decodable {
    struct Pickup: Decodable {
        var code: String
        var name: String
    }
    var canEdit: Bool
    var pickupChoices: [Pickup]
    var pickupSelected: String?
    var delayUntil: String?
    var notRequiredAfter: String?
    var message: String?

    var model: EditOptions {
        EditOptions(canEdit: canEdit, pickupChoices: pickupChoices.map { PickupChoice(code: $0.code, name: $0.name) },
                    pickupSelected: pickupSelected, delayUntil: delayUntil, notRequiredAfter: notRequiredAfter,
                    message: message)
    }
}
