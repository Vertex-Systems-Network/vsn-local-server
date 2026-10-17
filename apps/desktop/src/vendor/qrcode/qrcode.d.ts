// Minimal typings for the vendored qrcode-generator (see ./VENDOR.md).
export interface QrCode {
  addData(data: string, mode?: string): void;
  make(): void;
  getModuleCount(): number;
  isDark(row: number, col: number): boolean;
}
declare function qrcode(typeNumber: number, errorCorrectionLevel: 'L' | 'M' | 'Q' | 'H'): QrCode;
export default qrcode;
