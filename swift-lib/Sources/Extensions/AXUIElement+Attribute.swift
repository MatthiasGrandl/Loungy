/*
 This source file is part of the Loungy open source project
 
 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License
 
 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

import ApplicationServices

extension AXUIElement {
    func attribute(for name: String) -> CFTypeRef? {
        var value: CFTypeRef?
        AXUIElementCopyAttributeValue(self, name as CFString, &value)
        
        return value
    }
}
