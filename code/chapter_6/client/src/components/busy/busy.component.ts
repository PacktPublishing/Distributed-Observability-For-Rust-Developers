import { Component, HostBinding, inject } from "@angular/core";
import { BusyService } from "./busy.service";

@Component({
    selector: "div.busy-indicator",
    templateUrl: "./busy.component.html",
    styleUrls: ["./busy.component.scss"],
    standalone: true
})
export class BusyIndicatorComponent {
    private busyService = inject(BusyService);
    
    @HostBinding("class.visible") get visible() {
        return this.busyService.busy();
    }

    get message() {
        return this.busyService.message();
    }

    get isVisible() {
        return this.busyService.busy();
    }
}
