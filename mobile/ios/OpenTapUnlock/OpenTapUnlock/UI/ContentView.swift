//
//  ContentView.swift
//  OpenTapUnlock
//
//  Premium, intuitive dark-mode SwiftUI interface with glassmorphism and vibrant gradients.
//

import SwiftUI

struct ContentView: View {
    @State private var selectedTab = 0

    var body: some View {
        TabView(selection: $selectedTab) {
            StatusView()
                .tabItem {
                    Image(systemName: "lock.shield.fill")
                    Text("Status")
                }
                .tag(0)

            ShortcutsGuideView()
                .tabItem {
                    Image(systemName: "hand.tap.fill")
                    Text("Back Tap")
                }
                .tag(1)

            QrScannerView()
                .tabItem {
                    Image(systemName: "qrcode.viewfinder")
                    Text("Pair PC")
                }
                .tag(2)
        }
        .accentColor(Color("AccentColor", bundle: nil) != Color.clear ? Color.mint : Color.green)
        .preferredColorScheme(.dark)
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}
