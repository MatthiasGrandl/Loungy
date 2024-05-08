/*
 This source file is part of the Loungy open source project

 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License

 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

import Foundation

struct Modifiers: Codable {
    var control: Bool = false
    var alt: Bool = false
    var shift: Bool = false
    var platform: Bool = false
    var function: Bool = false
}
