import SwiftUI

@main
struct LibreSpydusApp: App {
    @State private var model: AppModel

    init() {
        Appearance.configure()
        let options = LaunchOptions.current
        let dependencies = AppModel.Dependencies.live(demo: options.demo)
        if options.reset {
            dependencies.resetPersistentState()
        }
        let model = AppModel(dependencies: dependencies)
        #if DEBUG
        options.apply(to: model)
        #endif
        _model = State(initialValue: model)
    }

    var body: some Scene {
        WindowGroup {
            RootView()
                .environment(model)
                .preferredColorScheme(.dark)
                .tint(Theme.Colors.primary)
                .dynamicTypeSize(...DynamicTypeSize.accessibility2)
        }
    }
}
