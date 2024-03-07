/*
 This source file is part of the Loungy open source project
 
 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License
 
 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

import Cocoa
import SwiftRs

public final class AppData: NSObject {
    var identifier: SRString
    var name: SRString
    
    init(identifier: String, name: String) {
        self.identifier = SRString(identifier)
        self.name = SRString(name)
    }
}
