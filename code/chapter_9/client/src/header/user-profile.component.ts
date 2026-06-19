import { Component, HostBinding, HostListener, signal, input } from '@angular/core';

@Component({
  selector: 'button.profile',
  templateUrl: './user-profile.component.html',
  styleUrls: ['./user-profile.component.scss'],
  standalone: true,
})
export class UserProfileComponent {
  userName = 'John Doe';
  userEmail = 'john.doe@example.com';
  @HostBinding('class.open') isActive = false;

  @HostListener('click', ['$event'])
  onProfileClick(event: MouseEvent): void {
    if (!this.isActive) {
      this.isActive = true;
    }
  }

  onClickOutside(clickedOutside: boolean): void {
    if (this.isActive && clickedOutside) {
      this.isActive = false;
    }
  }
}
