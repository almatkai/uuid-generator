/// <reference types="@raycast/api">

/* 🚧 🚧 🚧
 * This file is auto-generated from the extension's manifest.
 * Do not modify manually. Instead, update the `package.json` file.
 * 🚧 🚧 🚧 */

/* eslint-disable @typescript-eslint/ban-types */

type ExtensionPreferences = {
  /** Options - Should letters be uppercased? */
  "upperCaseLetters": boolean,
  /** Default Action - Set the default action */
  "defaultAction": "copy" | "paste"
}

/** Preferences accessible in all the extension's commands */
declare type Preferences = ExtensionPreferences

declare namespace Preferences {
  /** Preferences accessible in the `generate` command */
  export type Generate = ExtensionPreferences & {}
  /** Preferences accessible in the `generateV5` command */
  export type GenerateV5 = ExtensionPreferences & {
  /** UUID v5 Namespace - The namespace for UUID v5 */
  "uuidNamespace": string
}
  /** Preferences accessible in the `generateV7` command */
  export type GenerateV7 = ExtensionPreferences & {
  /** Options - Should letters be Base32 encoded? */
  "base32Encoding": boolean
}
  /** Preferences accessible in the `generateUlid` command */
  export type GenerateUlid = ExtensionPreferences & {}
  /** Preferences accessible in the `generateTypeID` command */
  export type GenerateTypeID = ExtensionPreferences & {}
  /** Preferences accessible in the `parseTypeID` command */
  export type ParseTypeID = ExtensionPreferences & {}
  /** Preferences accessible in the `formatUuid` command */
  export type FormatUuid = ExtensionPreferences & {}
  /** Preferences accessible in the `packTypeID` command */
  export type PackTypeID = ExtensionPreferences & {}
  /** Preferences accessible in the `viewHistory` command */
  export type ViewHistory = ExtensionPreferences & {}
}

declare namespace Arguments {
  /** Arguments passed to the `generate` command */
  export type Generate = {
  /** 1 */
  "numberOfUUIDsToGenerate": string
}
  /** Arguments passed to the `generateV5` command */
  export type GenerateV5 = {
  /** UUID Name */
  "name": string,
  /** 1 */
  "numberOfUUIDsToGenerate": string
}
  /** Arguments passed to the `generateV7` command */
  export type GenerateV7 = {
  /** 1 */
  "numberOfUUIDsToGenerate": string
}
  /** Arguments passed to the `generateUlid` command */
  export type GenerateUlid = {
  /** 1 */
  "numberOfULIDsToGenerate": string
}
  /** Arguments passed to the `generateTypeID` command */
  export type GenerateTypeID = {
  /** Prefix */
  "prefix": string,
  /** 1 */
  "numberOfUUIDsToGenerate": string
}
  /** Arguments passed to the `parseTypeID` command */
  export type ParseTypeID = {
  /** prefix_1234567890 */
  "typeID": string
}
  /** Arguments passed to the `formatUuid` command */
  export type FormatUuid = {
  /** 550e8400e29b41d4a716446655440000 */
  "uuid": string
}
  /** Arguments passed to the `packTypeID` command */
  export type PackTypeID = {
  /** Suffix */
  "suffix": string,
  /** UUID */
  "uuid": string
}
  /** Arguments passed to the `viewHistory` command */
  export type ViewHistory = {}
}

