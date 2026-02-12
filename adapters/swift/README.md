# Swift adapter

We intend to publish Swift adapters (CocoaPods + SwiftPM) under `adapters/swift/cocoapods` and `adapters/swift/spm`, but the current repository only contains placeholder files (`libforge.rb` and `Plugin.swift`). No Swift adapter is available yet. When implemented, the adapter will read `libforge.yaml`, download the signed manifest/artifacts, and surface the native libraries to CocoaPods/SwiftPM clients without rebuilding Rust locally.
