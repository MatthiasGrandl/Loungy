import Cocoa
import CoreGraphics
import SwiftRs

func getFocused(app: AXUIElement, prev: String) -> (AXUIElement, String)? {
    var focusedElementValue: AnyObject?

    let result = AXUIElementCopyAttributeValue(app, kAXFocusedUIElementAttribute as CFString, &focusedElementValue)
    if result != .success {
        return nil
    }

    let axElement = focusedElementValue as! AXUIElement

    var roleValue: AnyObject?
    _ = AXUIElementCopyAttributeValue(axElement, kAXRoleAttribute as CFString, &roleValue)
    let key = String(axElement.hashValue)

    if let role = roleValue as? String, role == kAXTextFieldRole || role == kAXTextAreaRole, key != prev {
        return (axElement, key)
    }

    return nil
}

@_cdecl("autofill")
public func autofill(value: SRString, prev: SRString) -> SRString? {
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
    var result: (AXUIElement, String)?
    repeat {
        result = getFocused(app: axApp, prev: prev)
        retries += 1
        if retries >= maxRetries {
            break
        }
        // Optionally, sleep or wait, if necessary, to avoid a busy loop
        // sleep(1) or Thread.sleep(forTimeInterval: 1)
        Thread.sleep(forTimeInterval: 0.1)
    } while result == nil

    guard let result = result else {
        print("autofill timed out")
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

@_cdecl("keytap")
func keytap(_ value: SRString) {
    // Wait for modifier keys to be released
    while areAnyModifierKeysPressed() {
        // Delaying the next modifier check to avoid tight looping.
        usleep(100_000) // Sleep for 100 milliseconds
    }

    // Type the password
    typePasswordWithAppleScript(value.toString())
}
