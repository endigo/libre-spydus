import SwiftUI

@main
struct LibreSpydusApp: App {
    @State private var model: AppModel
    @Environment(\.scenePhase) private var scenePhase
    /// Alerts show as banners in the app and a tap opens Your Books (§5.8). The notification centre
    /// holds its delegate weakly, so the app keeps it.
    private let notificationRouter: AlertNotificationRouter

    init() {
        Appearance.configure()
        TimingLog.install()
        // The morning check's handler (below) is registered later in the launch.
        SystemMorningCheckScheduler.holdUntilLaunched()
        let options = LaunchOptions.current
        let dependencies = AppModel.Dependencies.live(demo: options.demo, persistDemoAccount: options.persistDemoAccount,
                                                      demoRefreshFails: options.demoRefreshFails,
                                                      demoNewReady: options.demoNewReady,
                                                      alertTime: options.alertTime ?? AlertContent.alertTime)
        if options.reset {
            dependencies.resetPersistentState()
        }
        let model = AppModel(dependencies: dependencies)
        notificationRouter = AlertNotificationRouter.install { [weak model] segment in
            model?.openYourBooks(on: segment)
        }
        #if DEBUG
        options.apply(to: model)
        #endif
        model.rescheduleMorningCheck()
        _model = State(initialValue: model)
    }

    var body: some Scene {
        let model = model
        WindowGroup {
            RootView()
                .environment(model)
                .preferredColorScheme(.dark)
                .tint(Theme.Colors.primary)
                .dynamicTypeSize(...DynamicTypeSize.accessibility2)
        }
        .onChange(of: scenePhase) { _, phase in
            if phase == .background { model.rescheduleMorningCheck() }
        }
        // The morning check (§5.8): iOS launches or wakes the app for it some time after 07:45.
        .backgroundTask(.appRefresh(SystemMorningCheckScheduler.identifier)) {
            await model.runMorningCheck()
        }
    }
}
