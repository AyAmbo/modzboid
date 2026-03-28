export type AppErrorKind =
  | 'Io'
  | 'Parse'
  | 'NotFound'
  | 'Database'
  | 'Validation'
  | 'Game';

export interface AppError {
  kind: AppErrorKind;
  message: string;
}
