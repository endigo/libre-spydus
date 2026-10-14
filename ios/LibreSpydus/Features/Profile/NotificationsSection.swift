import SwiftUI
import UIKit

/// Profile → Notifications (docs/native-apps.md §5.8), shown while signed in: "Ready to collect"
/// and "Due-date reminders", app-wide and off until turned on. Turning one on asks for permission;
/// refused, it stays off and a line under the section says so, with Open Settings.
struct NotificationsSection: View {
    @Environment(AppModel.self) private var model
    @Environment(\.openURL) private var openURL
    @Environment(\.scenePhase) private var scenePhase

    var body: some View {
        let alerts = model.alerts
        VStack(alignment: .leading, spacing: Theme.Spacing.m) {
            ProfileSectionHeader(title: Copy.Notifications.section)
            ProfileCard {
                ProfileSwitchRow(title: Copy.Notifications.readyTitle, subtitle: Copy.Notifications.readySubtitle,
                               systemImage: "tray.and.arrow.down",
                               isOn: switchBinding(alerts.readyEnabled || alerts.asking == .ready) { on in
                                   await model.setReadyAlerts(on)
                               })
                ProfileDivider()
                ProfileSwitchRow(title: Copy.Notifications.dueTitle, subtitle: Copy.Notifications.dueSubtitle,
                               systemImage: "calendar.badge.clock",
                               isOn: switchBinding(alerts.dueEnabled || alerts.asking == .due) { on in
                                   await model.setDueAlerts(on)
                               })
            }
            .disabled(alerts.asking != nil)
            if alerts.permissionDenied {
                HStack(alignment: .firstTextBaseline, spacing: Theme.Spacing.s) {
                    Text(Copy.Notifications.blocked)
                        .themeFont(.caption)
                        .foregroundStyle(Theme.Colors.warning)
                        .fixedSize(horizontal: false, vertical: true)
                        .frame(maxWidth: .infinity, alignment: .leading)
                    Button(Copy.Notifications.openSettings) { openSettings() }
                        .themeFont(.bodyStrong)
                        .foregroundStyle(Theme.Colors.primary)
                        .buttonStyle(.borderless)
                }
                .padding(.horizontal, Theme.Spacing.xs)
            }
        }
        .task { await alerts.refreshPermission() }
        .onChange(of: scenePhase) { _, phase in
            // Back from Settings: the line goes once notifications are allowed.
            if phase == .active { Task { await alerts.refreshPermission() } }
        }
    }

    /// Shows `isOn`; a flip runs `set` (which may ask for permission and leave it off).
    private func switchBinding(_ isOn: Bool, set: @escaping @MainActor (Bool) async -> Void) -> Binding<Bool> {
        Binding(get: { isOn }, set: { on in Task { await set(on) } })
    }

    private func openSettings() {
        if let url = URL(string: UIApplication.openNotificationSettingsURLString) { openURL(url) }
    }
}

/// One switch on Profile: icon, title and a line saying what it does.
struct ProfileSwitchRow: View {
    let title: String
    let subtitle: String
    let systemImage: String
    @Binding var isOn: Bool

    var body: some View {
        Toggle(isOn: $isOn) {
            HStack(alignment: .top, spacing: Theme.Spacing.m) {
                Image(systemName: systemImage)
                    .font(.system(size: 17, weight: .semibold))
                    .foregroundStyle(Theme.Colors.primary)
                    .frame(width: 24)
                    .padding(.top, 1)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: Theme.Spacing.xs) {
                    Text(title)
                        .themeFont(.bodyStrong)
                        .foregroundStyle(Theme.Colors.textPrimary)
                    Text(subtitle)
                        .themeFont(.caption)
                        .foregroundStyle(Theme.Colors.textSecondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
        }
        .tint(Theme.Colors.success)
    }
}
