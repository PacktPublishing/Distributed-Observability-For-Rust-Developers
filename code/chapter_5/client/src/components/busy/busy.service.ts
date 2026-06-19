import { Injectable, signal } from "@angular/core";
import { Observable, finalize } from "rxjs";

@Injectable({ providedIn: 'root' })
export class BusyService {
    private static instance: BusyService;
    readonly busy = signal<boolean>(false);
    readonly message = signal<string>('Loading...');

    constructor() {
        BusyService.instance = this;
    }
    
    public static get Instance(): BusyService {
        // Simple guard for early access, though providedIn: 'root' should prevent this.
        if (!BusyService.instance) {
            throw new Error('BusyService not initialized. Ensure it is provided in root.');
        }
        return BusyService.instance;
    }

    show(message: string = 'Loading...'): void {
        this.message.set(message);
        this.busy.set(true);
    }

    hide(): void {
        this.busy.set(false);
    }
}