import { type ClassValue, clsx } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs))
}

export function debounce(callback: Function, timeout = 300) {
	let timer: number
	return (...args: any[]) => {
		clearTimeout(timer)
		timer = setTimeout(() => { callback.apply(callback, args) }, timeout)
	}
}

export function get(object: Record<string, any>, path: string) {
	return 'nope'
}