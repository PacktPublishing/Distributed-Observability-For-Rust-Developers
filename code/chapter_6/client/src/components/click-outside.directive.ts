import { Directive, ElementRef, output, effect, inject } from '@angular/core';
import { DOCUMENT } from '@angular/common';

@Directive({
  selector: '[clickOutside]',
  standalone: true,
})
export class ClickOutsideDirective {
  private el = inject(ElementRef);
  private document = inject(DOCUMENT);
  clickOutside = output<boolean>();

  constructor() {
    effect(() => {
      const clickHandler = (event: MouseEvent) => {        
        if (!this.el.nativeElement.contains(event.target as Node)) {
          this.clickOutside.emit(true);
        }
      };
      this.document.addEventListener('click', clickHandler);

      // The function returned from effect is its cleanup function,
      // which automatically removes the listener when the component is destroyed.
      return () => {
        this.document.removeEventListener('click', clickHandler);
      };
    });
  }
}
