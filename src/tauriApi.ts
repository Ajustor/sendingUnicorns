
// This file was generated by [tauri-specta](https://github.com/oscartbeaumont/tauri-specta). Do not edit this file manually.

/** user-defined commands **/


export const commands = {
async makeApiCall(method: string, url: string, options: RequestParams, bodyType: BodyTypesEnum) : Promise<Result<string, string>> {
    try {
    return { status: "ok", data: await TAURI_INVOKE("make_api_call", { method, url, options, bodyType }) };
} catch (e) {
    if(e instanceof Error) throw e;
    else return { status: "error", error: e  as any };
}
},
async createCollection(collectionName: string, config: CollectionConfig) : Promise<void> {
    await TAURI_INVOKE("create_collection", { collectionName, config });
},
async getCollections() : Promise<CollectionConfig[]> {
    return await TAURI_INVOKE("get_collections");
},
async updateCollection(collectionName: string, config: CollectionConfig) : Promise<void> {
    await TAURI_INVOKE("update_collection", { collectionName, config });
},
async exportCollection(collectionName: string) : Promise<void> {
    await TAURI_INVOKE("export_collection", { collectionName });
}
}

/** user-defined events **/



/** user-defined constants **/



/** user-defined types **/

export type BodyTypes = { json: string; form_data: ([string, Options])[] }
export type BodyTypesEnum = "Json" | "FormData"
export type CollectionConfig = { name: string; requests: Request[]; environments: Environment[] | null }
export type DynamicValue = string | number | boolean | null
export type Environment = { name: string; id: string; vars: ([string, string])[] }
export type JsonValue = null | boolean | number | string | JsonValue[] | { [key in string]: JsonValue }
export type Options = { is_active: boolean; value: DynamicValue }
export type Request = { name: string; url: string; method: string; id: string; pre_request_script: string | null; test: string | null; options: RequestOptions }
export type RequestOptions = { body: BodyTypes; params: ([string, Options])[]; headers: ([string, Options])[] }
export type RequestParams = { body: BodyTypes | null; params: ([string, JsonValue])[]; headers: ([string, string])[] }

/** tauri-specta globals **/

import {
	invoke as TAURI_INVOKE,
	Channel as TAURI_CHANNEL,
} from "@tauri-apps/api/core";
import * as TAURI_API_EVENT from "@tauri-apps/api/event";
import { type WebviewWindow as __WebviewWindow__ } from "@tauri-apps/api/webviewWindow";

type __EventObj__<T> = {
	listen: (
		cb: TAURI_API_EVENT.EventCallback<T>,
	) => ReturnType<typeof TAURI_API_EVENT.listen<T>>;
	once: (
		cb: TAURI_API_EVENT.EventCallback<T>,
	) => ReturnType<typeof TAURI_API_EVENT.once<T>>;
	emit: null extends T
		? (payload?: T) => ReturnType<typeof TAURI_API_EVENT.emit>
		: (payload: T) => ReturnType<typeof TAURI_API_EVENT.emit>;
};

export type Result<T, E> =
	| { status: "ok"; data: T }
	| { status: "error"; error: E };

function __makeEvents__<T extends Record<string, any>>(
	mappings: Record<keyof T, string>,
) {
	return new Proxy(
		{} as unknown as {
			[K in keyof T]: __EventObj__<T[K]> & {
				(handle: __WebviewWindow__): __EventObj__<T[K]>;
			};
		},
		{
			get: (_, event) => {
				const name = mappings[event as keyof T];

				return new Proxy((() => {}) as any, {
					apply: (_, __, [window]: [__WebviewWindow__]) => ({
						listen: (arg: any) => window.listen(name, arg),
						once: (arg: any) => window.once(name, arg),
						emit: (arg: any) => window.emit(name, arg),
					}),
					get: (_, command: keyof __EventObj__<any>) => {
						switch (command) {
							case "listen":
								return (arg: any) => TAURI_API_EVENT.listen(name, arg);
							case "once":
								return (arg: any) => TAURI_API_EVENT.once(name, arg);
							case "emit":
								return (arg: any) => TAURI_API_EVENT.emit(name, arg);
						}
					},
				});
			},
		},
	);
}
