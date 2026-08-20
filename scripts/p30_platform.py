#!/usr/bin/env python3
import platform

def canonical_platform(value=None):
    raw=(value if value is not None else platform.system()).strip().lower()
    return {'darwin':'macos','mac':'macos','macos':'macos','osx':'macos','win32':'windows','windows':'windows','cygwin':'windows','msys':'windows','linux':'linux'}.get(raw,raw)
