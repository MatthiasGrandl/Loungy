/*
 This source file is part of the Loungy open source project

 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License

 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

import Foundation
import SwiftRs
import Cocoa

// MARK: - Container Library

func enableAccessibilityFeatures() {
    let options: NSDictionary = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true]
    let accessibilityEnabled = AXIsProcessTrustedWithOptions(options)

    if accessibilityEnabled {
        print("Accessibility features enabled.")
    } else {
        print("Accessibility features not enabled. Please enable in System Preferences.")
    }
}


final class Library {
    let apps: Apps
    let autofill: AutoFill
    let ocr: OCR

    static let shared = Library()

    init() {
        enableAccessibilityFeatures()
        apps = Apps()
        autofill = AutoFill()
        ocr = OCR()
    }
}

// MARK: - Exposed Methods

let library = Library.shared
let apps = library.apps
let autofill = library.autofill
let ocr = library.ocr

// MARK: - Apps Public Methods

@_cdecl("get_frontmost_application_data")
public func frontmostApplicationData(cacheDir: SRString) -> AppData? {
    apps.frontmostApplicationData(
        for: cacheDir.toString()
    )
}

@_cdecl("get_application_data")
public func applicationData(cacheDir: SRString, path: SRString) -> AppData? {
    apps.applicationData(
        for: path.toString(),
        using: cacheDir.toString()
    )
}

// MARK: - AutoFill Public Methods

@_cdecl("autofill")
public func valuesToFill(value: SRString, password: Bool, prev: SRString) -> SRString? {
    guard let value = autofill.autofill(
        value: value.toString(),
        password: password,
        prev: prev.toString()
    )
    else {
        return nil
    }

    return SRString(value)
}

@_cdecl("paste")
func paste(value: SRString, formatting: Bool) {
    autofill.paste(
        value: value.toString(),
        formatting: formatting
    )
}

@_cdecl("copy_file")
func copyFile(for path: SRString) {
    autofill.copyFile(
        for: path.toString()
    )
}

@_cdecl("paste_file")
func pasteFile(path: SRString) {
    autofill.pasteFile(
        for: path.toString()
    )
}

@_cdecl("simulate_paste_event")
func simulatePasteEvent(formatting: Bool = true) {
    autofill.simulatePasteEvent(
        formatting: formatting
    )
}

// MARK: - MenuBar Public Methods

@_cdecl("menu_items")
public func listMenuItems() -> SRData {
    // TODO: Inject option to print debug messages on console
    do {
        let menubar = try MenuBar()
        let listItems = try menubar.listItems()

        return SRData(listItems)
    }
    catch {
        // TODO: Return a String containing the error message to present it in the Toaster
        error.printCustomInfo()
        // exit(0)
    }

    return SRData([])
}

@_cdecl("menu_item_select")
public func selectMenuItem(data: SRData) {
    do {
        let menubar = try MenuBar()

        try menubar.selectMenu(
            from: Data(data.toArray())
        )
    }
    catch {
        error.printCustomInfo()
        exit(0)
    }
}

// MARK: - OCR Public Methods

@_cdecl("ocr")
public func readText(path: SRString) {
    ocr.readText(
        from: path.toString()
    )
}
