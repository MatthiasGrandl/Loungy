/*
 This source file is part of the Loungy open source project
 
 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License
 
 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

import Cocoa
import CoreGraphics

extension NSBitmapImageRep {

    func png() -> Data? {
        representation(
            using: .png,
            properties: [:]
        )
    }

    func png() -> String? {
        guard 
            let data = representation(
                using: .png,
                properties: [:]
            )
        else {
            return nil
        }
        
        return data.base64EncodedString(
            options: []
        )
    }
}
