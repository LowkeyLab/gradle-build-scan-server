import { Routes } from '@angular/router';

export const routes: Routes = [
  { path: '', redirectTo: '/scans', pathMatch: 'full' },
  {
    path: 'scans',
    loadComponent: () =>
      import('./scans/scan-list.component').then(m => m.ScanListComponent),
  },
  {
    path: 'scans/:id',
    loadComponent: () =>
      import('./scans/scan-detail.component').then(m => m.ScanDetailComponent),
  },
];
