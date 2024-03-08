/*
 This source file is part of the Loungy open source project
 
 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License
 
 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

import Carbon.HIToolbox
import Cocoa
import CoreGraphics

final class AutoFill {

    func autofill(value: String, password: Bool, prev: String) -> String? {
        guard let currentApp = NSWorkspace.shared.frontmostApplication else {
            return nil
        }
        
        let axApp = AXUIElementCreateApplication(currentApp.processIdentifier)
        _ = AXUIElementSetAttributeValue(axApp, "AXEnhancedUserInterface" as CFString, true as CFTypeRef)
        _ = AXUIElementSetAttributeValue(axApp, "AXManualAccessibility" as CFString, true as CFTypeRef)

        guard let result = focusedElement(
            from: axApp,
            password: password,
            prev: prev
        )
        else {
            return nil
        }
        
        AXUIElementSetAttributeValue(
            result.element,
            kAXValueAttribute as CFString,
            value as CFTypeRef
        )
        
        return result.value
    }
    
    func paste(value: String, formatting: Bool) {
        let pasteboard = NSPasteboard.general
        pasteboard.declareTypes([.string], owner: nil)
        pasteboard.setString(value, forType: .string)
        
        simulatePasteEvent(formatting: formatting)
    }

    func copyFile(for path: String) {
        let pasteboard = NSPasteboard.general
        pasteboard.declareTypes([.fileURL], owner: nil)
        
        let path = URL(fileURLWithPath: path)
        pasteboard.writeObjects([path as NSPasteboardWriting])
    }

    func pasteFile(for path: String) {
        copyFile(for: path)
        simulatePasteEvent()
    }

    func simulatePasteEvent(formatting: Bool = true) {
        let sourceRef = CGEventSource(stateID: .combinedSessionState)
        
        // Create the Cmd Down event (Cmd is the Command key on macOS)
        guard 
            let cmdKeyDownEvent = CGEvent(
                keyboardEventSource: sourceRef,
                virtualKey: CGKeyCode(kVK_Command),
                keyDown: true
            )
        else {
            return
        }
        
        // Set the Command flag to emulate holding down the Cmd key
        cmdKeyDownEvent.flags = .maskCommand
        if !formatting {
            cmdKeyDownEvent.flags.insert(.maskShift)
            cmdKeyDownEvent.flags.insert(.maskAlternate)
        }
        
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
        
        if !formatting {
            // Create the Shift Up event
            if let shiftKeyUpEvent = CGEvent(keyboardEventSource: sourceRef, virtualKey: CGKeyCode(kVK_Shift), keyDown: false) {
                shiftKeyUpEvent.post(tap: .cghidEventTap)
            }
            
            // Create the Option Up event
            if let optionKeyUpEvent = CGEvent(keyboardEventSource: sourceRef, virtualKey: CGKeyCode(kVK_Option), keyDown: false) {
                optionKeyUpEvent.post(tap: .cghidEventTap)
            }
        }
    }
}

// MARK: - Private Methods

private extension AutoFill {
    
    func focusedElement(
        from app: AXUIElement,
        password: Bool,
        prev: String
    ) -> (element: AXUIElement, value: String)? {
        var focusedElementValue: AnyObject?
        
        let result = AXUIElementCopyAttributeValue(
            app,
            kAXFocusedUIElementAttribute as CFString,
            &focusedElementValue
        )
        
        guard result == .success else {
            return nil
        }
        
        let axElement = focusedElementValue as! AXUIElement
        
        var roleValue: AnyObject?
        _ = AXUIElementCopyAttributeValue(axElement, kAXRoleAttribute as CFString, &roleValue)
        
        var subroleValue: AnyObject?
        _ = AXUIElementCopyAttributeValue(axElement, kAXSubroleAttribute as CFString, &subroleValue)
        
        let passwordCheck = !password || (password && subroleValue as? String == kAXSecureTextFieldSubrole)
        
        let key = String(axElement.hashValue)
        
        if
            let role = roleValue as? String,
            role == kAXTextFieldRole || role == kAXTextAreaRole,
            passwordCheck,
            key != prev
        {
            return (axElement, key)
        }
        
        return nil
    }
}
