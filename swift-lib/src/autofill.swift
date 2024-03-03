import Carbon.HIToolbox
import Cocoa
import CoreGraphics
import SwiftRs

func getFocused(app: AXUIElement, password: Bool, prev: String) -> (AXUIElement, String)? {
    var focusedElementValue: AnyObject?

    let result = AXUIElementCopyAttributeValue(app, kAXFocusedUIElementAttribute as CFString, &focusedElementValue)
    if result != .success {
        return nil
    }

    let axElement = focusedElementValue as! AXUIElement

    var roleValue: AnyObject?
    _ = AXUIElementCopyAttributeValue(axElement, kAXRoleAttribute as CFString, &roleValue)

    var subroleValue: AnyObject?
    _ = AXUIElementCopyAttributeValue(axElement, kAXSubroleAttribute as CFString, &subroleValue)

    let passwordCheck = !password || (password && subroleValue as? String == kAXSecureTextFieldSubrole)

    let key = String(axElement.hashValue)

    if let role = roleValue as? String, role == kAXTextFieldRole || role == kAXTextAreaRole, passwordCheck, key != prev {
        return (axElement, key)
    }

    return nil
}

@_cdecl("autofill")
public func autofill(value: SRString, password: Bool, prev: SRString) -> SRString? {
    guard let currentApp = NSWorkspace.shared.frontmostApplication else {
        return nil
    }
    let axApp = AXUIElementCreateApplication(currentApp.processIdentifier)
    _ = AXUIElementSetAttributeValue(axApp, "AXEnhancedUserInterface" as CFString, true as CFTypeRef)
    _ = AXUIElementSetAttributeValue(axApp, "AXManualAccessibility" as CFString, true as CFTypeRef)

    let value = value.toString()
    let prev = prev.toString()

    let maxRetries = 1200
    var retries = 0
    let result = getFocused(app: axApp, password: password, prev: prev)

    guard let result = result else {
        return nil
    }

    AXUIElementSetAttributeValue(result.0, kAXValueAttribute as CFString, value as CFTypeRef)

    return SRString(result.1)
}

func areAnyModifierKeysPressed() -> Bool {
    let flags = NSEvent.modifierFlags
    return flags.contains(.shift) ||
        flags.contains(.control) ||
        flags.contains(.option) ||
        flags.contains(.command) ||
        flags.contains(.help) ||
        flags.contains(.function)
}

func typePasswordWithAppleScript(_ value: String) {
    let escapedValue = value.replacingOccurrences(of: "\"", with: "\\\"")

    let script =
        """
        tell application "System Events"
            keystroke "\(escapedValue)"
        end tell
        """

    var error: NSDictionary?
    if let scriptObject = NSAppleScript(source: script) {
        scriptObject.executeAndReturnError(&error)
        if let executionError = error {
            print("Failed to type password with error: \(executionError)")
        }
    }
}

@_cdecl("paste")
func paste(value: SRString) {
    let pasteboard = NSPasteboard.general
    pasteboard.declareTypes([.string], owner: nil)
    pasteboard.setString(value.toString(), forType: .string)
    simulatePasteEvent()
}

func simulatePasteEvent() {
    let sourceRef = CGEventSource(stateID: .combinedSessionState)

    // Create the Cmd Down event (Cmd is the Command key on macOS)
    if let cmdKeyDownEvent = CGEvent(keyboardEventSource: sourceRef, virtualKey: CGKeyCode(kVK_Command), keyDown: true) {
        // Set the Command flag to emulate holding down the Cmd key
        cmdKeyDownEvent.flags = .maskCommand

        // Create the 'V' key Down event
        if let vKeyDownEvent = CGEvent(keyboardEventSource: sourceRef, virtualKey: CGKeyCode(kVK_ANSI_V), keyDown: true) {
            vKeyDownEvent.flags = .maskCommand
            vKeyDownEvent.post(tap: .cghidEventTap)
        }

        // Create the 'V' key Up event
        if let vKeyUpEvent = CGEvent(keyboardEventSource: sourceRef, virtualKey: CGKeyCode(kVK_ANSI_V), keyDown: false) {
            vKeyUpEvent.flags = .maskCommand
            vKeyUpEvent.post(tap: .cghidEventTap)
        }

        // Create the Cmd Up event
        if let cmdKeyUpEvent = CGEvent(keyboardEventSource: sourceRef, virtualKey: CGKeyCode(kVK_Command), keyDown: false) {
            cmdKeyUpEvent.post(tap: .cghidEventTap)
        }
    }
}
