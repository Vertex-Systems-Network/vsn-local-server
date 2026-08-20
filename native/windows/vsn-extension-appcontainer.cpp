#define NOMINMAX
#include <windows.h>
#include <userenv.h>
#include <sddl.h>
#include <objbase.h>
#include <filesystem>
#include <iostream>
#include <string>
#include <thread>
#include <vector>
#include <algorithm>
#include <stdexcept>
#pragma comment(lib, "userenv.lib")
#pragma comment(lib, "advapi32.lib")
namespace fs = std::filesystem;

static std::wstring widen(const std::string& value) {
    if (value.empty()) return {};
    int n = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(), (int)value.size(), nullptr, 0);
    if (n <= 0) throw std::runtime_error("invalid UTF-8 argument");
    std::wstring out((size_t)n, L'\0');
    if (MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(), (int)value.size(), out.data(), n) != n) throw std::runtime_error("UTF-8 conversion failed");
    return out;
}
static std::wstring quote_arg(const std::wstring& arg) {
    if (arg.find_first_of(L" \t\"") == std::wstring::npos) return arg;
    std::wstring out=L"\""; size_t slashes=0;
    for (wchar_t c:arg) { if (c==L'\\'){++slashes;continue;} if(c==L'\"'){out.append(slashes*2+1,L'\\');out.push_back(L'\"');slashes=0;continue;} out.append(slashes,L'\\');slashes=0;out.push_back(c); }
    out.append(slashes*2,L'\\');out.push_back(L'\"');return out;
}
struct Options { std::wstring root, exec, profile; DWORD timeout=30000; bool network=false; std::vector<std::wstring> args; };
static Options parse(int argc,char** argv){Options o;for(int i=1;i<argc;++i){std::string k=argv[i];auto next=[&](){if(i+1>=argc)throw std::runtime_error("missing option value");return widen(argv[++i]);};if(k=="--root")o.root=next();else if(k=="--exec")o.exec=next();else if(k=="--profile")o.profile=next();else if(k=="--timeout")o.timeout=(DWORD)std::stoul(argv[++i])*1000;else if(k=="--network")o.network=std::string(argv[++i])=="1";else if(k=="--arg")o.args.push_back(next());else throw std::runtime_error("unknown option");}if(o.root.empty()||o.exec.empty()||o.profile.empty())throw std::runtime_error("root/exec/profile required");o.timeout=std::clamp<DWORD>(o.timeout,1000,125000);return o;}
static HRESULT ensure_profile(const std::wstring& name, PSID* sid){HRESULT hr=CreateAppContainerProfile(name.c_str(),L"VSN Extension Sandbox",L"VSN isolated extension process",nullptr,0,sid);if(hr==HRESULT_FROM_WIN32(ERROR_ALREADY_EXISTS))hr=DeriveAppContainerSidFromAppContainerName(name.c_str(),sid);return hr;}
static PSID internet_client_sid(){PSID* groups=nullptr;PSID* caps=nullptr;DWORD gc=0,cc=0;if(!DeriveCapabilitySidsFromName(L"internetClient",&groups,&gc,&caps,&cc)||cc<1){if(groups){for(DWORD i=0;i<gc;++i)if(groups[i])LocalFree(groups[i]);LocalFree(groups);}if(caps){for(DWORD i=0;i<cc;++i)if(caps[i])LocalFree(caps[i]);LocalFree(caps);}return nullptr;}for(DWORD i=0;i<gc;++i)if(groups[i])LocalFree(groups[i]);if(groups)LocalFree(groups);PSID result=caps[0];for(DWORD i=1;i<cc;++i)if(caps[i])LocalFree(caps[i]);LocalFree(caps);return result;}
static std::vector<char> read_pipe(HANDLE h,size_t limit){std::vector<char> out;char buf[8192];DWORD n=0;while(ReadFile(h,buf,sizeof(buf),&n,nullptr)&&n){if(out.size()+n>limit)break;out.insert(out.end(),buf,buf+n);}CloseHandle(h);return out;}
int main(int argc,char** argv){
    if(argc==2&&std::string(argv[1])=="--probe") return 0;
    PSID appSid=nullptr, netSid=nullptr;LPPROC_THREAD_ATTRIBUTE_LIST attrs=nullptr;HANDLE outR=nullptr,outW=nullptr,errR=nullptr,errW=nullptr,nul=nullptr;PROCESS_INFORMATION pi{};fs::path runRoot;
    try{
        Options o=parse(argc,argv);if(FAILED(ensure_profile(o.profile,&appSid))||!appSid)throw std::runtime_error("AppContainer profile creation failed");
        PWSTR folder=nullptr;if(FAILED(GetAppContainerFolderPath(o.profile.c_str(),&folder))||!folder)throw std::runtime_error("AppContainer folder lookup failed");fs::path container(folder);CoTaskMemFree(folder);
        runRoot=container/L"VSN"/(L"run-"+std::to_wstring(GetCurrentProcessId())+L"-"+std::to_wstring(GetTickCount64()));fs::create_directories(runRoot);fs::copy(fs::path(o.root),runRoot,fs::copy_options::recursive|fs::copy_options::overwrite_existing);
        fs::path rel=o.exec.lexically_normal();if(rel.is_absolute()||std::find(rel.begin(),rel.end(),fs::path(L".."))!=rel.end())throw std::runtime_error("unsafe relative executable path");fs::path exe=fs::weakly_canonical(runRoot/rel);fs::path canonRoot=fs::weakly_canonical(runRoot);std::error_code relEc;fs::path contained=fs::relative(exe,canonRoot,relEc);if(relEc||contained.empty()||contained.is_absolute()||std::find(contained.begin(),contained.end(),fs::path(L".."))!=contained.end()||!fs::is_regular_file(exe))throw std::runtime_error("executable escapes staged AppContainer package");
        SID_AND_ATTRIBUTES cap{};SECURITY_CAPABILITIES security{};security.AppContainerSid=appSid;if(o.network){netSid=internet_client_sid();if(!netSid)throw std::runtime_error("internetClient capability derivation failed");cap.Sid=netSid;cap.Attributes=SE_GROUP_ENABLED;security.Capabilities=&cap;security.CapabilityCount=1;}
        SIZE_T attrSize=0;InitializeProcThreadAttributeList(nullptr,1,0,&attrSize);std::vector<unsigned char> attrBuf(attrSize);attrs=(LPPROC_THREAD_ATTRIBUTE_LIST)attrBuf.data();if(!InitializeProcThreadAttributeList(attrs,1,0,&attrSize))throw std::runtime_error("InitializeProcThreadAttributeList failed");if(!UpdateProcThreadAttribute(attrs,0,PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,&security,sizeof(security),nullptr,nullptr))throw std::runtime_error("AppContainer security attribute setup failed");
        SECURITY_ATTRIBUTES sa{sizeof(SECURITY_ATTRIBUTES),nullptr,TRUE};if(!CreatePipe(&outR,&outW,&sa,0)||!CreatePipe(&errR,&errW,&sa,0))throw std::runtime_error("stdout/stderr pipe creation failed");SetHandleInformation(outR,HANDLE_FLAG_INHERIT,0);SetHandleInformation(errR,HANDLE_FLAG_INHERIT,0);nul=CreateFileW(L"NUL",GENERIC_READ,FILE_SHARE_READ|FILE_SHARE_WRITE,&sa,OPEN_EXISTING,FILE_ATTRIBUTE_NORMAL,nullptr);
        STARTUPINFOEXW si{};si.StartupInfo.cb=sizeof(si);si.lpAttributeList=attrs;si.StartupInfo.dwFlags=STARTF_USESTDHANDLES;si.StartupInfo.hStdInput=nul;si.StartupInfo.hStdOutput=outW;si.StartupInfo.hStdError=errW;
        std::wstring cmd=quote_arg(exe.wstring());for(auto& a:o.args){cmd.push_back(L' ');cmd+=quote_arg(a);}std::vector<wchar_t> mutableCmd(cmd.begin(),cmd.end());mutableCmd.push_back(L'\0');
        if(!CreateProcessW(exe.c_str(),mutableCmd.data(),nullptr,nullptr,TRUE,EXTENDED_STARTUPINFO_PRESENT|CREATE_NO_WINDOW,nullptr,runRoot.c_str(),&si.StartupInfo,&pi))throw std::runtime_error("CreateProcessW AppContainer launch failed");CloseHandle(outW);outW=nullptr;CloseHandle(errW);errW=nullptr;
        std::vector<char> stdoutData,stderrData;std::thread outThread([&]{stdoutData=read_pipe(outR,2*1024*1024);outR=nullptr;});std::thread errThread([&]{stderrData=read_pipe(errR,512*1024);errR=nullptr;});DWORD wait=WaitForSingleObject(pi.hProcess,o.timeout);if(wait==WAIT_TIMEOUT){TerminateProcess(pi.hProcess,124);WaitForSingleObject(pi.hProcess,5000);}DWORD code=125;GetExitCodeProcess(pi.hProcess,&code);CloseHandle(pi.hThread);CloseHandle(pi.hProcess);pi={};outThread.join();errThread.join();std::cout.write(stdoutData.data(),(std::streamsize)stdoutData.size());std::cerr.write(stderrData.data(),(std::streamsize)stderrData.size());if(attrs){DeleteProcThreadAttributeList(attrs);attrs=nullptr;}if(nul){CloseHandle(nul);nul=nullptr;}if(netSid){LocalFree(netSid);netSid=nullptr;}if(appSid){FreeSid(appSid);appSid=nullptr;}std::error_code ec;fs::remove_all(runRoot,ec);return wait==WAIT_TIMEOUT?124:(int)code;
    }catch(const std::exception& e){std::cerr<<"vsn AppContainer sandbox error: "<<e.what()<<"\n";if(pi.hThread)CloseHandle(pi.hThread);if(pi.hProcess){TerminateProcess(pi.hProcess,125);CloseHandle(pi.hProcess);}if(outW)CloseHandle(outW);if(errW)CloseHandle(errW);if(outR)CloseHandle(outR);if(errR)CloseHandle(errR);if(nul)CloseHandle(nul);if(attrs)DeleteProcThreadAttributeList(attrs);if(netSid)LocalFree(netSid);if(appSid)FreeSid(appSid);std::error_code ec;if(!runRoot.empty())fs::remove_all(runRoot,ec);return 125;}
}
