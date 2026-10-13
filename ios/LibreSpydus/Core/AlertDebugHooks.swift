#if DEBUG
import Foundation
import LibreSpydusCore

/// `-LSDemoAlert <kind>` (DEBUG, demo mode only; README "Debug launch hooks"): once the launch
/// refresh has settled, one alert from the demo account is scheduled `demoAlertDelay` seconds
/// ahead, built by the same `AlertContent` code as the real ones (payload included), so tapping it
/// from a banner or Notification Centre can be tried without waiting for 07:45. Asks for
/// notification permission first (once).
extension AppModel {
    /// Time to leave the app before the alert shows.
    static let demoAlertDelay: TimeInterval = 8

    func postDemoAlert(_ kind: LaunchOptions.DemoAlert) async {
        guard isDemo, let snapshot = account.snapshot else { return }
        let system = alerts.system
        guard await system.notifications.requestPermission() else {
            AlertLog.print("demo alert: notifications aren't allowed")
            return
        }
        let origin = backend.origin
        let calendar = system.calendar()
        let now = system.now()
        let reminders = dueReminders(snapshot: snapshot, today: AlertContent.simpleDate(now, calendar: calendar),
                                     daysBefore: AlertContent.dueDaysBefore)
        let notification: AlertNotification?
        switch kind {
        case .ready:
            notification = snapshot.reservations.ready.items.first.map { AlertContent.readyNotification($0, origin: origin) }
        case .due:
            notification = reminders.first { $0.items.count == 1 }.map { AlertContent.dueNotification($0, origin: origin) }
        case .dueSeveral:
            // The demo loans are all due on different days: one reminder naming every loan due from today on.
            notification = reminders.first.map { first in
                AlertContent.dueNotification(DueReminder(key: first.key, due: first.due, remindOn: first.remindOn,
                                                         items: reminders.flatMap(\.items)), origin: origin)
            }
        }
        guard var notification else {
            AlertLog.print("demo alert: nothing to say for \(kind.rawValue)")
            return
        }
        notification.at = calendar.dateComponents([.calendar, .year, .month, .day, .hour, .minute, .second],
                                                   from: now.addingTimeInterval(Self.demoAlertDelay))
        // One already delivered with this ID would only be updated, without a banner.
        await system.notifications.remove(pending: [notification.id], delivered: [notification.id])
        await system.notifications.add(notification)
    }
}
#endif
