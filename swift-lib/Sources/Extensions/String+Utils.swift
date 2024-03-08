/*
 This source file is part of the Loungy open source project
 
 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License
 
 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

extension String {
    var isInt: Bool {
        Int(self) != nil
    }
    
    var isNotEmpty: Bool {
        isEmpty == false
    }
    
    func clearNewLines() -> String {
        replacingOccurrences(of: "\n", with: " ").trimmingCharacters(in: .whitespaces)
    }
    
    mutating func clearedNewLines() {
        self = clearNewLines()
    }
}
