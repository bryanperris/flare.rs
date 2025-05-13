/*
tOMMSHashNode *Osiris_OMMS_FindHashNode(char *script_name, bool autocreate);
tOMMSHashNode *Osiris_OMMS_DeleteHashNode(tOMMSHashNode *node);
tOMMSNode *Osiris_OMMS_FindNode(tOMMSHashNode *root, unsigned int uid, bool autocreate);
void Osiris_OMMS_RemoveNode(tOMMSHashNode *root, unsigned int uid);
void Osiris_OMMS_ReduceRefCount(tOMMSHashNode *root, tOMMSNode *node);
void Osiris_OMMS_CallFreeForNode(tOMMSHashNode *root, tOMMSNode *node);
tOMMSNode *Osiris_OMMS_FindHandle(OMMSHANDLE handle, tOMMSHashNode **hash = NULL);
OMMSHANDLE Osiris_OMMS_Malloc(size_t amount_of_memory, uint unique_identifier, char *script_identifier);
void *Osiris_OMMS_Attach(OMMSHANDLE handle);
void Osiris_OMMS_Detach(OMMSHANDLE handle);
void Osiris_OMMS_Free(OMMSHANDLE handle);
OMMSHANDLE Osiris_OMMS_Find(uint unique_identifier, char *script_identifier);
char Osiris_OMMS_GetInfo(OMMSHANDLE handle, uint *mem_size, uint *uid, ushort *reference_count,  ubyte *has_free_been_called);
void Osiris_InitOMMS(void)
void Osiris_CloseOMMS(void) 
void Osiris_SaveOMMS(CFILE *file)
void Osiris_RestoreOMMS(CFILE *file) 
tOMMSHashNode *Osiris_OMMS_FindHashNode(char *script_name, bool autocreate)
tOMMSHashNode *Osiris_OMMS_DeleteHashNode(tOMMSHashNode *node)
tOMMSNode *Osiris_OMMS_FindNode(tOMMSHashNode *root, unsigned int uid, bool autocreate)
void Osiris_OMMS_RemoveNode(tOMMSHashNode *root, unsigned int uid)
tOMMSNode *Osiris_OMMS_FindHandle(OMMSHANDLE handle, tOMMSHashNode **hash)
OMMSHANDLE Osiris_OMMS_Malloc(size_t amount_of_memory, uint unique_identifier, char *script_identifier) 
void *Osiris_OMMS_Attach(OMMSHANDLE handle) 
void Osiris_OMMS_Detach(OMMSHANDLE handle) 
void Osiris_OMMS_Free(OMMSHANDLE handle) 
OMMSHANDLE Osiris_OMMS_Find(uint unique_identifier, char *script_identifier) 
char Osiris_OMMS_GetInfo(OMMSHANDLE handle, uint *mem_size, uint *uid, ushort *reference_count, ubyte *has_free_been_called) 
void Osiris_CreateModuleInitStruct(tOSIRISModuleInit *mi) {

 */