/*
 This source file is part of the Loungy open source project
 
 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License
 
 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

import Cocoa
import CoreGraphics

final class Apps {

    func frontmostApplicationData(for cacheDir: String) -> AppData? {
        guard let currentApp = NSWorkspace.shared.frontmostApplication else {
            return nil
        }

        guard
            let identifier = currentApp.bundleIdentifier,
            let bundle = Bundle(identifier: identifier),
            let path = bundle.executablePath
        else {
            return nil
        }
        
        guard let name = bundle.name else {
            return nil
        }
        
        let data = AppData(
            identifier: identifier,
            name: name
        )
        
        if saveImage(cacheDir: cacheDir, path: path, bundleIdentifier: identifier) == false {
            return nil
        }
        
        return data
    }
    
    func applicationData(for path: String, using cacheDir: String) -> AppData? {
        guard
            let bundle = Bundle(path: path),
            let identifier = bundle.bundleIdentifier,
            let name = bundle.name
        else {
            return nil
        }
        
        if let extensionPointIdentifier = bundle.extensionPointIdentifier,
            extensionPointIdentifier != "com.apple.Settings.extension.ui" {
            return nil
        }
        
        let data = AppData(
            identifier: identifier,
            name: name
        )
        
        guard 
            saveImage(
                cacheDir: cacheDir,
                path: path,
                bundleIdentifier: identifier
            )
        else {
            return nil
        }
        
        return data
    }
}

// MARK: - Private Methods

private extension Apps {
    
    /// Make `print()` accept an array of items.
    /// Since Swift doesn't support spreading...
    func print<Target>(
        _ items: [Any],
        separator: String = " ",
        terminator: String = "\n",
        to output: inout Target
    ) where Target: TextOutputStream {
        let item = items.map { "\($0)" }.joined(separator: separator)
        
        Swift.print(
            item,
            terminator: terminator,
            to: &output
        )
    }
    
    func icon(for path: String, size: Int) -> Data? {
        NSWorkspace
            .shared
            .icon(forFile: path)
            .resize(to: size)
            .png()
    }
    
    func saveImage(cacheDir: String, path: String, bundleIdentifier: String) -> Bool {
        let fileManager = FileManager.default
        let fileURL = URL(
            fileURLWithPath: cacheDir,
            isDirectory: true
        ).appendingPathComponent("\(bundleIdentifier).png")
        
        guard fileManager.fileExists(atPath: fileURL.path) == false else {
            return true
        }
        
        guard let iconData = icon(for: path, size: 128) else {
            return false
        }
        
        do {
            try iconData.write(to: fileURL)
            return true
        }
        catch {
            return false
        }
    }
}
