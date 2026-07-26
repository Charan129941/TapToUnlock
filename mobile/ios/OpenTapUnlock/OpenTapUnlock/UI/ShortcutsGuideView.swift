//
//  ShortcutsGuideView.swift
//  OpenTapUnlock
//
//  Clean, intuitive 3-step walkthrough on linking Apple iOS Back Tap in iPhone Settings.
//

import SwiftUI

struct ShortcutsGuideView: View {
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(alignment: .leading, spacing: 24) {
                    Text("How to Enable System-Wide Back Tap")
                        .font(.title2)
                        .fontWeight(.bold)
                        .foregroundColor(.white)

                    Text("Follow these 3 simple steps in your iPhone Settings to unlock your laptop from any app without draining battery:")
                        .font(.subheadline)
                        .foregroundColor(.gray)

                    StepCard(
                        stepNumber: "1",
                        title: "Open Accessibility Settings",
                        description: "Go to iPhone Settings ➔ Accessibility ➔ Touch ➔ scroll down and tap 'Back Tap'."
                    )

                    StepCard(
                        stepNumber: "2",
                        title: "Select Triple Tap or Double Tap",
                        description: "Choose whether 3 taps or 2 taps on the back of your phone will trigger the unlock."
                    )

                    StepCard(
                        stepNumber: "3",
                        title: "Assign 'Unlock My Workstation'",
                        description: "Scroll down to the 'Shortcuts' section and tap 'Unlock My Workstation'. That's it!"
                    )

                    VStack(spacing: 12) {
                        Image(systemName: "hand.tap.fill")
                            .font(.largeTitle)
                            .foregroundColor(.mint)
                        Text("You're All Set!")
                            .font(.headline)
                            .fontWeight(.bold)
                            .foregroundColor(.white)
                        Text("Now, whenever your phone is awake and unlocked by Face ID, simply tap the back of your iPhone to open your PC screen instantly.")
                            .font(.caption)
                            .foregroundColor(.gray)
                            .multilineTextAlignment(.center)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(20)
                    .background(
                        RoundedRectangle(cornerRadius: 16)
                            .fill(Color(UIColor.secondarySystemBackground))
                    )
                }
                .padding(20)
            }
            .navigationTitle("Back Tap Setup")
        }
    }
}

struct StepCard: View {
    let stepNumber: String
    let title: String
    let description: String

    var body: some View {
        HStack(alignment: .top, spacing: 16) {
            Text(stepNumber)
                .font(.title2)
                .fontWeight(.bold)
                .frame(width: 36, height: 36)
                .background(Color.mint)
                .foregroundColor(.black)
                .clipShape(Circle())

            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .font(.headline)
                    .fontWeight(.bold)
                    .foregroundColor(.white)
                Text(description)
                    .font(.subheadline)
                    .foregroundColor(.gray)
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 14)
                .fill(Color(UIColor.secondarySystemBackground))
        )
    }
}
