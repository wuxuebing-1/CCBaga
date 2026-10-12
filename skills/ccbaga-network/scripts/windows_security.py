"""Windows-only private file creation/validation and child-process containment.

Uses the standard library and Win32 APIs. ACLs are applied at creation, before
any secret bytes are written; chmod on Windows is not a confidentiality control.
"""
import ctypes as c
from ctypes import wintypes as w
import msvcrt
import os

k = c.WinDLL('kernel32', use_last_error=True)
a = c.WinDLL('advapi32', use_last_error=True)
P = c.c_void_p


def api(dll, name, result, *args):
    fn = getattr(dll, name)
    fn.restype, fn.argtypes = result, args
    return fn


close = api(k, 'CloseHandle', w.BOOL, w.HANDLE)
free = api(k, 'LocalFree', P, P)
api(k, 'GetCurrentProcess', w.HANDLE)
api(a, 'OpenProcessToken', w.BOOL, w.HANDLE, w.DWORD, c.POINTER(w.HANDLE))
api(a, 'GetTokenInformation', w.BOOL, w.HANDLE, c.c_int, P, w.DWORD, c.POINTER(w.DWORD))
api(a, 'ConvertSidToStringSidW', w.BOOL, P, c.POINTER(w.LPWSTR))
api(a, 'ConvertStringSecurityDescriptorToSecurityDescriptorW', w.BOOL,
    w.LPCWSTR, w.DWORD, c.POINTER(P), P)
api(a, 'GetSecurityInfo', w.DWORD, w.HANDLE, c.c_int, w.DWORD,
    c.POINTER(P), P, c.POINTER(P), P, c.POINTER(P))
api(a, 'GetAce', w.BOOL, P, w.DWORD, c.POINTER(P))


def checked(ok):
    if not ok:
        raise c.WinError(c.get_last_error())
    return ok


def sid_text(pointer):
    text = w.LPWSTR()
    checked(a.ConvertSidToStringSidW(pointer, c.byref(text)))
    try:
        return text.value
    finally:
        free(c.cast(text, P))


def current_sid():
    token, size = w.HANDLE(), w.DWORD()
    checked(a.OpenProcessToken(k.GetCurrentProcess(), 8, c.byref(token)))
    try:
        a.GetTokenInformation(token, 1, None, 0, c.byref(size))
        data = c.create_string_buffer(size.value)
        checked(a.GetTokenInformation(token, 1, data, size, c.byref(size)))
        return sid_text(c.cast(data, c.POINTER(P))[0])
    finally:
        close(token)


class Attributes(c.Structure):
    _fields_ = [('length', w.DWORD), ('descriptor', P), ('inherit', w.BOOL)]


api(k, 'CreateFileW', w.HANDLE, w.LPCWSTR, w.DWORD, w.DWORD,
    c.POINTER(Attributes), w.DWORD, w.DWORD, w.HANDLE)


def private_fd(path):
    descriptor = P()
    sid = current_sid()
    checked(a.ConvertStringSecurityDescriptorToSecurityDescriptorW(
        f'O:{sid}D:P(A;;FA;;;{sid})', 1, c.byref(descriptor), None))
    try:
        attributes = Attributes(c.sizeof(Attributes), descriptor, False)
        # CREATE_NEW, no sharing and no following final reparse points.
        handle = k.CreateFileW(str(path), 0x40000000, 0, c.byref(attributes), 1, 0x00200080, None)
        if handle == w.HANDLE(-1).value:
            code = c.get_last_error()
            if code in (80, 183):
                raise FileExistsError(str(path))
            raise c.WinError(code)
        try:
            return msvcrt.open_osfhandle(handle, os.O_WRONLY | os.O_BINARY)
        except BaseException:
            close(handle)
            raise
    finally:
        free(descriptor)


def validate_private_fd(fd):
    owner, dacl, descriptor = P(), P(), P()
    code = a.GetSecurityInfo(msvcrt.get_osfhandle(fd), 1, 5, c.byref(owner),
                             None, c.byref(dacl), None, c.byref(descriptor))
    if code:
        raise c.WinError(code)
    try:
        sid = current_sid()
        if sid_text(owner) != sid or not dacl:
            raise ValueError('Private file must be owned by the current Windows user and have a restricted ACL')
        # ACL header: revision, spare, size, ACE count, spare.
        count = c.cast(dacl, c.POINTER(c.c_ushort))[2]
        for i in range(count):
            ace = P()
            checked(a.GetAce(dacl, i, c.byref(ace)))
            ace_type = c.cast(ace, c.POINTER(c.c_ubyte))[0]
            # Standard access-allowed ACE: 4-byte header, mask, then SID.
            if ace_type != 0 or sid_text(ace.value + 8) not in (sid, 'S-1-5-18', 'S-1-5-32-544'):
                raise ValueError('Private file ACL grants access outside its owner, SYSTEM and Administrators')
    finally:
        free(descriptor)


class BasicLimits(c.Structure):
    _fields_ = [('process_time', c.c_longlong), ('job_time', c.c_longlong),
                ('flags', w.DWORD), ('min_working', c.c_size_t), ('max_working', c.c_size_t),
                ('active', w.DWORD), ('affinity', c.c_size_t), ('priority', w.DWORD), ('scheduling', w.DWORD)]


class ExtendedLimits(c.Structure):
    _fields_ = [('basic', BasicLimits), ('io', c.c_ulonglong * 6),
                ('process_memory', c.c_size_t), ('job_memory', c.c_size_t),
                ('peak_process', c.c_size_t), ('peak_job', c.c_size_t)]


api(k, 'CreateJobObjectW', w.HANDLE, P, w.LPCWSTR)
api(k, 'SetInformationJobObject', w.BOOL, w.HANDLE, c.c_int, P, w.DWORD)
api(k, 'AssignProcessToJobObject', w.BOOL, w.HANDLE, w.HANDLE)


class Job:
    def __init__(self, process):
        self.handle = checked(k.CreateJobObjectW(None, None))
        limits = ExtendedLimits()
        limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        try:
            checked(k.SetInformationJobObject(self.handle, 9, c.byref(limits), c.sizeof(limits)))
            checked(k.AssignProcessToJobObject(self.handle, int(process._handle)))
        except BaseException:
            self.close()
            process.kill()
            process.wait()
            raise

    def close(self):
        if self.handle:
            close(self.handle)
            self.handle = None
