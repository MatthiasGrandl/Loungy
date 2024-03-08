/*
 This source file is part of the Loungy open source project
 
 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License
 
 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

import Cocoa
import Foundation

extension Bundle {

    var name: String? {
        var name: String?
        
        if let displayName, displayName.isEmpty == false {
            name = displayName
        }
        else if let bundleName, bundleName.isEmpty == false {
            name = bundleName
        }
        else if let executableName, executableName.isEmpty == false {
            name = executableName
        }
        
        return name
    }

    var extensionPointIdentifier: String? {
        extensionAttributes?["EXExtensionPointIdentifier"] as? String
    }
}

// MARK: - Privates

private extension Bundle {

    func contentValue(for keyName: String) -> String? {
        let localizedName = localizedInfoDictionary?[keyName] as? String
        let dictionaryName = infoDictionary?[keyName] as? String
        
        var name: String?
        
        if let localizedName {
            name = localizedName
        }
        else if let dictionaryName {
            name = dictionaryName
        }
        
        return name
    }

    var extensionAttributes: [String: Any]? {
        infoDictionary?["EXAppExtensionAttributes"] as? [String: Any]
    }
    
    var executableName: String? {
        contentValue(for: "CFBundleExecutable")
    }
    
    var displayName: String? {
        contentValue(for: "CFBundleDisplayName")
    }
    
    var bundleName: String? {
        contentValue(for: "CFBundleName")
    }
}
