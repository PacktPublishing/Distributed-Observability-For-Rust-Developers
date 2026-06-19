import { finalize, Observable, tap } from 'rxjs';
import { BusyService } from './busy.service';
import { inject } from '@angular/core';

export interface BusyOptions {
  message?: string;
  showBusy?: boolean;
}

export function Busy(options: BusyOptions = { showBusy: true }): MethodDecorator {
  return function (
    target: any,
    propertyKey: string | symbol,
    descriptor: PropertyDescriptor
  ) {
    // Store the original method implementation
    const originalMethod = descriptor.value;

    // Replace the original method with a new function
    descriptor.value = function (this: any, ...args: any[]): Observable<any> {
      const busyService = BusyService.Instance;
      const { message = 'Loading...', showBusy = true } = options;

      // Call the original method to get the Observable
      const result: Observable<any> = originalMethod.apply(this, args);

      // If showBusy is false, return the original observable
      if (!showBusy) {
        return result;
      }

      // Wrap with busy indicator
      return result.pipe(
        tap(() => busyService.show(message)),
        finalize(() => busyService.hide())
      );
    };

    return descriptor; // Return the modified descriptor
  };
}