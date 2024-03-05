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

@_cdecl("get_frontmost_application_data")
public func getFrontmostApplicationData(cacheDir: SRString) -> AppData? {
    guard let currentApp = NSWorkspace.shared.frontmostApplication else {
        return nil
    }
    guard let bundle = Bundle(identifier: currentApp.bundleIdentifier!) else {
        return nil
    }
    guard let bundleId = bundle.bundleIdentifier else {
        return nil
    }
    guard let path = bundle.executablePath else {
        return nil
    }
    guard let name = bundle.name() else {
        return nil
    }
    let data = AppData(bundleId, name)

    if !saveImage(cacheDir: cacheDir, path: path, bundleId: bundleId) {
        return nil
    }

    return data
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
    if !saveImage(cacheDir: cacheDir, path: path.toString(), bundleId: bundleId) {
        return nil
    }

    return data
}

func saveImage(cacheDir: SRString, path: String, bundleId: String) -> Bool {
    let url = NSURL(fileURLWithPath: cacheDir.toString(), isDirectory: true)
    if let pathComponent = url.appendingPathComponent("\(bundleId).png") {
        let filePath = pathComponent.path
        let fileManager = FileManager.default
        if fileManager.fileExists(atPath: filePath) {
            return true
        } else {
            guard let icon: Data = getIcon(path: path, size: 128) else {
                return false
            }
            do {
                try icon.write(to: pathComponent)
            } catch {
                return false
            }
        }
    } else {
        return false
    }
    return true
}
