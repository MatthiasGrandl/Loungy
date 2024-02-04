import Cocoa
import CoreGraphics
import SwiftRs

extension NSBitmapImageRep {
    func png() -> Data? {
        representation(using: .png, properties: [:])
    }
}

extension NSBitmapImageRep {
    func png() -> String? {
        guard let pngData = representation(using: .png, properties: [:]) else {
            return nil
        }
        return pngData.base64EncodedString(options: [])
    }
}

extension Data {
    var bitmap: NSBitmapImageRep? { NSBitmapImageRep(data: self) }
}

extension NSImage {
    func png() -> Data? { tiffRepresentation?.bitmap?.png() }

    func resizedForFile(to size: Int) -> NSImage {
        let newSizeInt = size / Int(NSScreen.main?.backingScaleFactor ?? 1)
        let newSize = CGSize(width: newSizeInt, height: newSizeInt)

        let image = NSImage(size: newSize)
        image.lockFocus()
        NSGraphicsContext.current?.imageInterpolation = .high

        draw(
            in: CGRect(origin: .zero, size: newSize),
            from: .zero,
            operation: .copy,
            fraction: 1
        )

        image.unlockFocus()
        return image
    }
}

extension FileHandle: TextOutputStream {
    public func write(_ string: String) {
        write(string.data(using: .utf8)!)
    }
}

extension String {
    var isInt: Bool { Int(self) != nil }
}

enum PrintOutputTarget {
    case standardOutput
    case standardError
}

/// Make `print()` accept an array of items.
/// Since Swift doesn't support spreading...
private func print<Target>(
    _ items: [Any],
    separator: String = " ",
    terminator: String = "\n",
    to output: inout Target
) where Target: TextOutputStream {
    let item = items.map { "\($0)" }.joined(separator: separator)
    Swift.print(item, terminator: terminator, to: &output)
}

func getIcon(path: String, size: Int) -> Data? {
    return NSWorkspace.shared.icon(forFile: path).resizedForFile(to: size).png()
}

extension Bundle {
    func name() -> String? {
        guard let name = localizedInfoDictionary?["CFBundleDisplayName"] as? String else {
            guard let name = infoDictionary?["CFBundleDisplayName"] as? String else {
                guard let name = localizedInfoDictionary?["CFBundleName"] as? String else {
                    guard let name = infoDictionary?["CFBundleName"] as? String else {
                        return nil
                    }
                    return name
                }
                return name
            }
            return name
        }
        return name
    }
}

public class AppData: NSObject {
    var id: SRString
    var name: SRString

    init(_ id: String, _ name: String) {
        self.id = SRString(id)
        self.name = SRString(name)
    }
}

@_cdecl("get_application_data")
public func getApplicationData(cacheDir: SRString, path: SRString) -> AppData? {
    guard let bundle = Bundle(path: path.toString()) else {
        return nil
    }
    guard let bundleId = bundle.bundleIdentifier else {
        return nil
    }
    guard let name = bundle.name() else {
        return nil
    }
    if let ex = bundle.infoDictionary?["EXAppExtensionAttributes"] as? [String: Any], let extensionPointIdentifier = ex["EXExtensionPointIdentifier"] as? String {
        if extensionPointIdentifier != "com.apple.Settings.extension.ui" {
            return nil
        }
    }

    let data = AppData(bundleId, name)

    let url = NSURL(fileURLWithPath: cacheDir.toString(), isDirectory: true)
    if let pathComponent = url.appendingPathComponent("\(bundleId).png") {
        let filePath = pathComponent.path
        let fileManager = FileManager.default
        if fileManager.fileExists(atPath: filePath) {
            return data
        } else {
            guard let icon: Data = getIcon(path: path.toString(), size: 128) else {
                return nil
            }
            do {
                try icon.write(to: pathComponent)
            } catch {
                return nil
            }
        }
    } else {
        return nil
    }
    return data
}

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
